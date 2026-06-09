/**
 * BR-VC-9 unknown-optional-feature disposition — the FF-4 decision-core.
 *
 * When a governed contract payload carries an **unknown optional feature**
 * (e.g. an additive field from a higher MINOR than this build understands),
 * the engine selects one of three dispositions, chosen by the effective
 * **run mode** and the **contract's class**:
 *
 *   - **Application** (default) — *per-contract*: tolerant on presentation
 *     contracts (templates, tokens), strict (reject) on data/security
 *     contracts (snapshot/`datadom`, privacy export, edge render-state).
 *   - **Build / SSR** — *strict*: reject every optional unknown across all
 *     contracts, so build artifacts / server-rendered output never silently
 *     drop content.
 *   - **Development** — *tolerant*: degrade (accept + continue, surfacing
 *     diagnostics) across all contracts, so work-in-progress is not
 *     hard-stopped.
 *
 * The **must-understand reject** of BR-VC-8 is a separate axis and holds in
 * **every** mode: a feature flagged must-understand always rejects,
 * regardless of the optional-feature disposition.
 *
 * This module is the pure decision-core only (BR-VC-9 §6.5 / AC-P-6.7).
 * Wiring it to the actual contract ingest paths (snapshot / edge-render-state
 * read-back) and threading the run mode from the host/build config are a
 * follow-up slice; this core carries no I/O and no global state so it is
 * deterministic and exhaustively unit-testable. It backs fitness function
 * FF-4 (`tools/fitness/fitness-gates.json`).
 */

/** Run mode selecting the unknown-optional-feature disposition (BR-VC-9). */
export type RunMode = 'application' | 'build-ssr' | 'development';

/**
 * Disposition applied to a single unknown OPTIONAL feature.
 * - `reject` — strict: the unknown feature is an error; the host must not
 *   silently proceed.
 * - `degrade` — tolerant: accept the payload without the unknown feature's
 *   semantics (treat as unvalidated/foreign) and continue, surfacing a
 *   diagnostic.
 * - `ignore` — tolerant: drop the unknown feature and continue, surfacing a
 *   diagnostic (a report event).
 */
export type Disposition = 'reject' | 'degrade' | 'ignore';

/**
 * BR-VC-9 contract classes. Presentation contracts tolerate unknown optional
 * features in an application run; data/security contracts do not.
 */
export type ContractClass = 'presentation' | 'data-security';

/**
 * Governed contracts that can carry versioned optional features (mirrors
 * `tools/fitness/governed-contracts.json` plus the BR-VC-9-named
 * `privacy-export`, which has no SemVer locator yet).
 */
export type GovernedContractId =
    | 'data-snapshot'
    | 'edge-render-state'
    | 'privacy-export'
    | 'template-authoring-cem-ml'
    | 'token-outputs'
    | 'patch-transport';

/**
 * Static BR-VC-9 classification of each governed contract.
 *
 * Data/security (strict in an application run): the snapshot/`datadom`
 * contract, privacy export, and edge render-state — these cross trust /
 * serialization boundaries, so an unrecognized optional feature must not be
 * silently honored or dropped. Presentation (tolerant): templates and
 * tokens. `patch-transport` is the render-plan/patch stream — rendering
 * output, classed presentation; it is not named explicitly by BR-VC-9, so
 * the conservative reviewer note lives here.
 */
const CONTRACT_CLASS: Readonly<Record<GovernedContractId, ContractClass>> = {
    'data-snapshot': 'data-security',
    'edge-render-state': 'data-security',
    'privacy-export': 'data-security',
    'template-authoring-cem-ml': 'presentation',
    'token-outputs': 'presentation',
    'patch-transport': 'presentation',
};

/** BR-VC-9 class of a governed contract. */
export function classifyContract(contract: GovernedContractId): ContractClass {
    return CONTRACT_CLASS[contract];
}

/** A single, auditable disposition decision (BR-VC-9 "record the active mode"). */
export interface DispositionDecision {
    mode: RunMode;
    contract: GovernedContractId;
    contractClass: ContractClass;
    disposition: Disposition;
    /** True when the unknown feature is rejected (strict outcome). */
    strict: boolean;
    /** True when the reject is forced by BR-VC-8 must-understand, not the optional-feature policy. */
    mustUnderstand: boolean;
    /** Human-readable rationale citing the governing rule. */
    rationale: string;
}

/** The tolerant disposition chosen per run mode (ignore/degrade per BR-VC-9). */
const TOLERANT_BY_MODE: Readonly<Record<RunMode, Exclude<Disposition, 'reject'>>> = {
    // An application run keeps presentation content but treats the unknown
    // optional feature as unvalidated — degrade rather than drop.
    application: 'degrade',
    // build-ssr is never tolerant (handled before this table is consulted).
    'build-ssr': 'degrade',
    // Development preserves work-in-progress: degrade + surface diagnostics.
    development: 'degrade',
};

/**
 * Decide the BR-VC-9 disposition for an unknown optional feature on
 * `contract` under `mode`. Pure and total over the typed domain.
 *
 * `mustUnderstand` (BR-VC-8): when the unknown feature is flagged
 * must-understand it rejects in every mode, ahead of the optional-feature
 * policy.
 */
export function decideDisposition(
    mode: RunMode,
    contract: GovernedContractId,
    opts: { mustUnderstand?: boolean } = {},
): DispositionDecision {
    const contractClass = classifyContract(contract);
    const mustUnderstand = opts.mustUnderstand === true;

    if (mustUnderstand) {
        return {
            mode,
            contract,
            contractClass,
            disposition: 'reject',
            strict: true,
            mustUnderstand: true,
            rationale: 'BR-VC-8: a must-understand feature rejects in every run mode.',
        };
    }

    // BR-VC-9 mode rules.
    let strict: boolean;
    let rationale: string;
    switch (mode) {
        case 'build-ssr':
            strict = true;
            rationale =
                'BR-VC-9: build/SSR rejects every optional unknown across all contracts.';
            break;
        case 'development':
            strict = false;
            rationale =
                'BR-VC-9: development tolerates (degrades) optional unknowns across all contracts, surfacing diagnostics.';
            break;
        case 'application':
        default:
            strict = contractClass === 'data-security';
            rationale = strict
                ? 'BR-VC-9: an application run rejects optional unknowns on data/security contracts.'
                : 'BR-VC-9: an application run tolerates (degrades) optional unknowns on presentation contracts.';
            break;
    }

    return {
        mode,
        contract,
        contractClass,
        disposition: strict ? 'reject' : TOLERANT_BY_MODE[mode],
        strict,
        mustUnderstand: false,
        rationale,
    };
}
