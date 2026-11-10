//! Compile validated SDL and fixed operations to the native JSON guard.
use apollo_compiler::{
    ast::{OperationType, Type},
    executable::{Selection, SelectionSet},
    parser::Parser,
    schema::ExtendedType,
    ExecutableDocument, Schema,
};
use cem_ml::validation::json::graphql::{
    Field, GraphqlContracts, OperationContract, OperationKind, Scalar, Shape,
};
use std::collections::BTreeMap;

pub const MAX_GRAPHQL_SOURCE_BYTES: usize = 256 * 1024;
const MAX_NODES: usize = 4096;
const MAX_DEPTH: usize = 24;

type Fields = BTreeMap<String, Field>;

pub fn compile_graphql_contracts(sdl: &str, source: &str) -> Result<GraphqlContracts, String> {
    if [sdl, source]
        .iter()
        .any(|text| text.len() > MAX_GRAPHQL_SOURCE_BYTES)
    {
        return Err("GraphQL build source exceeds 256 KiB".into());
    }
    let mut parser = Parser::new().recursion_limit(64).token_limit(16_384);
    let schema = parser
        .parse_schema(sdl, "schema.graphql")
        .map_err(|e| e.to_string())?
        .validate()
        .map_err(|e| e.to_string())?;
    let document = parser
        .parse_executable(&schema, source, "operations.graphql")
        .map_err(|e| e.to_string())?
        .validate(&schema)
        .map_err(|e| e.to_string())?;
    if document.operations.anonymous.is_some()
        || !(1..=64).contains(&document.operations.named.len())
    {
        return Err("GraphQL contracts require 1 to 64 named operations".into());
    }
    let mut compiler = Compiler {
        schema: &schema,
        document: &document,
        remaining: MAX_NODES,
    };
    let mut operations = BTreeMap::new();
    for (name, operation) in &document.operations.named {
        reject_directives(operation.directives.is_empty())?;
        let kind = match operation.operation_type {
            OperationType::Query => OperationKind::Query,
            OperationType::Mutation => OperationKind::Mutation,
            OperationType::Subscription => {
                return Err("GraphQL subscriptions need a stream contract".into())
            }
        };
        let mut variables = Fields::new();
        for variable in &operation.variables {
            reject_directives(variable.directives.is_empty())?;
            variables.insert(
                bounded_name(&variable.name)?,
                compiler.input_field(&variable.ty, variable.default_value.is_some(), 0)?,
            );
        }
        let mut data = Fields::new();
        compiler.selection_fields(&operation.selection_set, &mut data, 0)?;
        operations.insert(
            bounded_name(name)?,
            OperationContract {
                kind,
                variables,
                data,
            },
        );
    }
    let contracts = GraphqlContracts::new(sdl.as_bytes(), source.as_bytes(), operations);
    contracts.pinned_bytes()?;
    Ok(contracts)
}

struct Compiler<'a> {
    schema: &'a Schema,
    document: &'a ExecutableDocument,
    remaining: usize,
}

impl Compiler<'_> {
    fn visit(&mut self, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH || self.remaining == 0 {
            return Err("GraphQL expanded shape exceeds its depth or node budget".into());
        }
        self.remaining -= 1;
        Ok(())
    }

    fn input_field(&mut self, ty: &Type, defaulted: bool, depth: usize) -> Result<Field, String> {
        Ok(Field {
            optional: defaulted || matches!(ty, Type::Named(_) | Type::List(_)),
            shape: lower_type(ty, depth, &mut |name, depth| {
                self.visit(depth)?;
                match self.schema.types.get(name) {
                    Some(ExtendedType::InputObject(input)) => {
                        if input.directives.has("oneOf") {
                            return Err("GraphQL OneOf input needs a dedicated contract".into());
                        }
                        let mut fields = Fields::new();
                        for (name, field) in &input.fields {
                            fields.insert(
                                bounded_name(name)?,
                                self.input_field(
                                    &field.ty,
                                    field.default_value.is_some(),
                                    depth + 1,
                                )?,
                            );
                        }
                        Ok(Shape::Object(fields))
                    }
                    _ => self.scalar_or_enum(name),
                }
            })?,
        })
    }

    fn selection_fields(
        &mut self,
        set: &SelectionSet,
        fields: &mut Fields,
        depth: usize,
    ) -> Result<(), String> {
        if !matches!(
            self.schema.types.get(&set.ty),
            Some(ExtendedType::Object(_))
        ) {
            return Err(format!(
                "GraphQL response selection requires a concrete object: {}",
                set.ty
            ));
        }
        for selection in &set.selections {
            self.visit(depth)?;
            match selection {
                Selection::Field(field) => {
                    reject_directives(field.directives.is_empty())?;
                    if field.name.starts_with("__") {
                        return Err("GraphQL introspection needs a dedicated contract".into());
                    }
                    let shape = lower_type(field.ty(), depth, &mut |name, depth| {
                        if field.selection_set.selections.is_empty() {
                            self.scalar_or_enum(name)
                        } else {
                            let mut nested = Fields::new();
                            self.selection_fields(&field.selection_set, &mut nested, depth + 1)?;
                            Ok(Shape::Object(nested))
                        }
                    })?;
                    merge_field(
                        fields,
                        bounded_name(field.response_key())?,
                        Field {
                            optional: false,
                            shape,
                        },
                    )?;
                }
                Selection::FragmentSpread(spread) => {
                    let fragment = &self.document.fragments[&spread.fragment_name];
                    reject_directives(
                        spread.directives.is_empty() && fragment.directives.is_empty(),
                    )?;
                    if fragment.selection_set.ty != set.ty {
                        return Err("GraphQL fragment must select the same concrete object".into());
                    }
                    self.selection_fields(&fragment.selection_set, fields, depth + 1)?;
                }
                Selection::InlineFragment(fragment) => {
                    reject_directives(fragment.directives.is_empty())?;
                    if fragment.selection_set.ty != set.ty {
                        return Err("GraphQL fragment must select the same concrete object".into());
                    }
                    self.selection_fields(&fragment.selection_set, fields, depth + 1)?;
                }
            }
        }
        Ok(())
    }
    fn scalar_or_enum(&mut self, name: &str) -> Result<Shape, String> {
        let scalar = match name {
            "Boolean" => Scalar::Boolean,
            "Float" => Scalar::Float,
            "ID" => Scalar::Id,
            "Int" => Scalar::Int,
            "String" => Scalar::String,
            _ => match self.schema.types.get(name) {
                Some(ExtendedType::Enum(enumeration)) => {
                    self.remaining = self
                        .remaining
                        .checked_sub(enumeration.values.len())
                        .ok_or("GraphQL expanded enum exceeds its node budget")?;
                    return Ok(Shape::Enum(
                        enumeration
                            .values
                            .keys()
                            .map(|name| bounded_name(name))
                            .collect::<Result<_, _>>()?,
                    ));
                }
                _ => {
                    return Err(format!(
                        "GraphQL type needs a reviewed runtime shape: {name}"
                    ))
                }
            },
        };
        Ok(Shape::Scalar(scalar))
    }
}

fn bounded_name(name: &str) -> Result<String, String> {
    if name.len() > 256 {
        return Err("GraphQL name exceeds 256 bytes".into());
    }
    Ok(name.into())
}

fn reject_directives(empty: bool) -> Result<(), String> {
    if empty {
        Ok(())
    } else {
        Err("GraphQL executable directives need a dedicated contract".into())
    }
}

fn lower_type(
    ty: &Type,
    depth: usize,
    named: &mut impl FnMut(&str, usize) -> Result<Shape, String>,
) -> Result<Shape, String> {
    if depth > MAX_DEPTH {
        return Err("GraphQL expanded shape exceeds its depth budget".into());
    }
    let shape = match ty {
        Type::Named(name) | Type::NonNullNamed(name) => named(name, depth)?,
        Type::List(item) | Type::NonNullList(item) => {
            Shape::List(Box::new(lower_type(item, depth + 1, named)?))
        }
    };
    if matches!(ty, Type::Named(_) | Type::List(_)) {
        Ok(Shape::Nullable(Box::new(shape)))
    } else {
        Ok(shape)
    }
}

fn merge_field(fields: &mut Fields, name: String, field: Field) -> Result<(), String> {
    use std::collections::btree_map::Entry;
    match fields.entry(name) {
        Entry::Vacant(entry) => {
            entry.insert(field);
            Ok(())
        }
        Entry::Occupied(mut entry) => merge_shape(&mut entry.get_mut().shape, field.shape),
    }
}

fn merge_shape(current: &mut Shape, additional: Shape) -> Result<(), String> {
    match (current, additional) {
        (Shape::Object(current), Shape::Object(additional)) => {
            for (name, field) in additional {
                merge_field(current, name, field)?;
            }
            Ok(())
        }
        (Shape::List(current), Shape::List(additional))
        | (Shape::Nullable(current), Shape::Nullable(additional)) => {
            merge_shape(current, *additional)
        }
        (current, additional) if *current == additional => Ok(()),
        _ => Err("GraphQL response has incompatible selections for one key".into()),
    }
}
