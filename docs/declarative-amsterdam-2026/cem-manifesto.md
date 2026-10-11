# CEM Manifesto

**Software should be easy to bring together—and able to change.**

CEM's purpose is to make data processing and software capabilities easier to
compose, reuse, and evolve. Its foundation is a shared declarative platform
for structured data, transformations, and application composition. It serves
automated pipelines, processing embedded in services and tools, and web
applications through explicit contracts for meaning, state, and resource
access. Embeddable applications extend that reuse into the places people work.

This manifesto states that direction. The [platform pitch](cem-platform-pitch.md)
addresses prospective users; the [presentation plan](presentation-plan.md)
records audience strategy, presentation design and adoption decisions. The
[evidence record](platform-evidence.md) distinguishes
available capabilities from the work needed to fulfill the whole vision.

## Carry definitions through the whole development lifecycle

Keep the development workflow connected. Choose sources. Pick, extend, or
create your own schemas. Develop modules, then compose, debug, test, package,
and run. The same definitions should guide development and govern the running
solution.

Schemas should remain useful as validation rules and development knowledge.
Transformations should be reusable modules with inspectable results. Runtime
diagnostics should lead back to the source and a reproducible check. Changing
a source, vendor, or deployment environment should preserve the contracts and
tests that still apply. This continuity makes accumulated development work
useful throughout operation and evolution.

## Make processing reusable across delivery channels

Data from several sources should be able to feed several useful outputs through
a common set of rules. A result may serve another pipeline, become a published
document or service response, or drive an interactive experience.

Validation, queries, and transformations should remain useful in a CLI job,
an embedded library, and a web host. Each host supplies its own delivery and
resource access. Teams should be able to add an output or change a consumer
without duplicating the processing rules that remain the same.

## Meet people where their work happens

A product becomes more useful when its capabilities can participate in another
product's workflow. A supplier's availability check belongs inside a buyer's
ordering experience. A publishing tool belongs beside the content it helps
create. An application should be able to travel with its useful behavior,
data contracts, and identity.

Embeddability should be part of application design. Requiring every partner to
reconstruct the same interaction wastes the effort of both the provider and
the host.

The provider and the host should be able to improve their products on separate
release schedules. Their integration needs a published contract that makes
compatibility assessable without requiring a shared framework or development
team.

## Let people keep their language and expertise

Teams already have documents, schemas, queries, stylesheets, and domain
knowledge. Those investments should become entry points into a larger platform.

XML and CEM-ML can express related structures. XPath and CEM-QL can ask related
questions. CSV, JSON, YAML, and other inputs can enter a shared processing
lifecycle through explicit mappings. A team should be able to choose a useful
authoring form while preserving the meaning that downstream work depends on.

Different syntaxes have different strengths. A common model must preserve
those distinctions and make any transformation or loss visible.

## Separate processing from data location

A processing rule should express what the data means and what result is
required. Input and output representations, resource locations, and execution
hosts should be explicit boundaries that can evolve around that rule.

A new partner format or a storage migration should preserve existing
processing wherever the agreed domain contract remains the same. Teams should
be able to build, extend, and customize connectors for their own sources and
destinations. Format adapters describe the representation; connectors use
host resolver and publication interfaces for access and delivery. Location
independence must preserve permission checks and source attribution.

## Make contracts reusable

A field's meaning, a validation rule, a transformation, and a presentation
should have clear owners. Shared contracts let many consumers use the same
definition and let its owner improve it deliberately.

CEM puts vocabularies and validation rules in schema packages, computation in
declared queries and transformations, and interfaces in reusable declarations.
New source syntaxes join through explicit adapters. Integration then becomes
work on visible boundaries that can be inspected, versioned, and tested.

## Make effects explicit

A useful view can begin with supplied data and a declared transformation.
Saving a decision, requesting more data, or invoking a business action adds
another responsibility: a host-granted service with an explicit interface.

This separation gives a team a small first adoption step and makes later
authority visible. It also lets a host supply persistence or actions without
rewriting the capability's presentation.

## Make change a normal operation

Applications outlive individual tools. Formats change, vendors change,
deployment targets change, and requirements change.

An evolutionary architecture identifies what may change and what must remain
true. For CEM, these enduring properties include document meaning, source
identity, explicit authority, state ownership, and observable failures.
Executable checks should protect them as implementations evolve.

A successful extension should leave unrelated parts of the system useful.
The practical test is a working replacement with a clear account of what
changed.

When a provider changes its data contract, an explicit compatibility
transformation can preserve an older consumer's agreed meaning. Such adapters
need versioned inputs, outputs and tests; changes that cannot preserve meaning
must remain visible to the consumer.

## Insulate every included scope

Independently supplied applications, pipeline stages, and connectors need
runtime boundaries around what they can access, affect, and consume. CEM's
scope architecture calls for security insulation across memory, storage, CPU,
DOM, CSS, and network access. Each applicable layer must constrain the actual
execution of a contribution.

Declarations express requested access. The host determines what to grant.
Enforcement mechanisms maintain the insulation, admitting authorized crossings
and rejecting unauthorized ones. The permission contract configures these
boundaries; their observable behavior establishes the protection.

Sharing a pipeline or a page must preserve each scope's private state,
permitted effects, and resource limits. A nested contribution stays within
its parent's safeguards and receives only authority its parent may delegate.
Its work counts against the parent's resource budget. Creating more scopes
must preserve that aggregate limit.

Replacing or revoking a contribution must retire its authority. Generated
output and resumed state must preserve the boundary when they become active.
This is a commitment to observable enforcement at every inclusion and effect.
It is how independently supplied software can become practical to compose
under the data and resource owner's terms.

## Let state survive movement and rendering

An application's state should have a durable identity beyond its current
rendered pixels. A view is a projection of that state.

The same state contract should support server-produced output, browser
continuation, and work delegated to another execution host. Moving state
must preserve its meaning while excluding credentials and host capabilities
that do not belong at the destination.

Load and activate work when it is needed. Make version, revision, and policy
checks explicit when an application resumes.

## Treat performance as a property of the whole lifecycle

Useful performance comes from the complete path from input to usable result:
reading, validation, computation, formatting, output, and, where needed,
rendering. Batch throughput and service latency matter alongside interactive
responsiveness.

Retained structures can avoid unnecessary conversions. Independent tasks can
run concurrently. Suitable stages can produce and consume results
progressively. Reuse can avoid repeated work. Budgets can keep one operation
from consuming every resource.

Each of these choices must be measured on real workloads. Time to a useful
result, responsiveness, memory, transfer, and total completion time all matter.

## Keep explanations attached to results

A developer should be able to ask where a value came from, why a rule rejected
it, and which transformation produced the view.

Source identity, diagnostics, and provenance are part of the platform's value.
They let people inspect abstractions, understand a partner's integration, and
repair a failure without reconstructing the whole pipeline.

## Make generated software reviewable

AI-assisted development increases the opportunity to create useful software.
Making that software dependable and usable inside other products remains
engineering work.

Declarative contracts offer a surface that both people and tools can inspect:
what data is expected, what output is produced, and what resources are
requested. Validation, behavior tests, and host enforcement still have distinct
jobs. The opportunity is to make those checks reusable as more software is
created.

## Earn adoption through a useful first step

A team should be able to start with one format boundary, one transformation,
or one embedded capability. Existing systems should remain useful while the
team learns where CEM helps.

CEM earns a broader role when it makes that first integration clearer, its next
change cheaper, and its behavior easier to explain.

**Bring your data. Keep your expertise. Build a capability others can compose.**
