# Granted element placements

Status: adopted design, 2026-10-07; implementation remains in
[todo.md](todo.md#next-consumer-actions). Current native export still requires
exactly one target placement in the requesting producer forest.

A native reference identifies an original CEM node. Crossing its source scope
requires the existing directed lifecycle grant. Using a DOM placement owned by
another producer additionally requires a host-issued placement grant. Neither
grant substitutes for the other. Matching IDs and local-name conveniences do
not issue grants or prove node identity.

Reference evaluation still uses the lifecycle host's explicit source contexts
and effective scope policies. A placement admission neither supplies that context
nor overwrites it with another instance's frame. Context roots need no IDs.
The registry maps evaluated terminal nodes to placements at consumption time;
it does not attach a shared target list to authored references.

## Ownership and admission

The embedding host owns a placement registry and a publication coordinator.
Each admitted placement binds a retained native node handle to its producer's
durable instance identity, stable placement path, committed render revision,
DOM root and concrete element. Host-owned elements outside cem-element output
have an explicit host producer identity and revision with the same obligations.
The registry retains the original source owner for the lifetime of the admission.
It never reconstructs node identity from a DOM record or document ID lookup.

A placement grant names the requesting instance, target producer, admitted node
and placement, and permitted relationship properties. It is directed and
revocable. It does not authorize subsequent nodes, replacement placements,
reverse relationships or additional consumers. A producer's durable identity
alone does not make later render revisions ready.

The coordinator presents an immutable, bounded snapshot to native export. A
snapshot entry carries an opaque admission token, retained node identity,
producer/path/revision and reserved DOM ID. Browser elements stay in the host's
registry; workers receive native handles and explicitly named control metadata.
No AST becomes a JavaScript record. Placement entries and checks consume the
consumer's existing work budget; destination traversal limits remain effective.

Exactly one accessible placement must match a terminal target. Multiple matching
local placements remain ambiguous even when only one has an ID. An external
placement cannot repair a local ambiguity. A node with a local placement uses
that placement; a grant is consulted only when no local placement exists.
Multiple admitted external placements require an explicit unique placement in
the grant, rather than a choice by output order or matching ID.

## Readiness and publication

A ready placement has a committed producer revision, a connected element in
the same permitted DOM root, a live grant and a unique valid ID. The owning
producer preserves its explicit ID or reserves a generated ID using its own
instance and placement path. The requesting consumer cannot mutate another
producer to generate an ID. An external host producer explicitly accepts that
same ID ownership obligation when registering its element.

Preparation resolves every relationship against one registry snapshot and
reserves all involved placements and IDs. Before commit, the coordinator checks
the requesting render token, admission revisions, grants, connectedness and ID
uniqueness again. A changed dependency makes preparation incomplete. No subset
of a replacement plan or its relationships is published.

For a ready, already committed producer, the coordinator can publish the
consumer alone. If several producers must introduce or change placements
together, a coordinated transaction stages all affected forests and relationships,
checks their combined ID space, and commits them together. Nested producers may
not acknowledge readiness until their concrete target placement has committed.
Cycles of preparation dependencies remain incomplete and receive a diagnostic;
they do not acquire authority by waiting or repeating preparation.

An incomplete ordinary replacement preserves the previous committed plan while
its existing dependencies remain valid. Revocation, disconnect or loss of the
admitted placement invalidates those existing dependent relationships immediately:
the coordinator removes the affected native ID attributes/markers and releases
provider claims as one cleanup operation. It does not preserve a revoked route
while waiting for a replacement. Unrelated committed content remains available.
Provider close/focus/geometry rules apply to this cleanup, without synthesizing a
new invocation or transferring the old context to another instance.

## Resume and implementation sequence

SSR may use host-issued admissions for placements committed in the same SSR
transaction. Serialized producer/path/revision descriptors are resume hints;
they convey no grants. Hydration obtains fresh admissions and validates live
elements before enabling relationships. Removing and reconnecting an instance
reuses its durable identity but requires fresh readiness and authority.

Implement the bounded native admission snapshot and negative fixtures first,
then the host coordinator and transactional browser publication, then SSR and
hydration re-admission. Until those actions are complete, targets outside the
current forest continue to produce the existing missing-placement result.
The four focus/geometry slots do not create a placement exception.
