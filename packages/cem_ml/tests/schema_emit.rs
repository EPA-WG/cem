//! Verification fixtures for the schema-compiler output module.
//!
//! Fixture file locations are declared by
//! `cem-ml-stack-design.md` §13.2.7 and
//! `cem-ml-stack-design-impl.md` §3.4.2.6. Each fixture lives in
//! `tests/schema_emit/` and pins a specific AC.

mod schema_emit {
    pub mod rng_compact_roundtrip;
    pub mod rng_xml_oracle;
    pub mod rust_hdr_compiles;
    pub mod ts_dts_structural;
    pub mod ts_dts_validated_brand;
    pub mod ts_fixture_support;
}
