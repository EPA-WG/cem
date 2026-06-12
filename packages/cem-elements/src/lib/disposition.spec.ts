import { describe, expect, it } from 'vitest';

import {
    classifyContract,
    decideDisposition,
    ingestContractVersion,
    type ContractClass,
    type GovernedContractId,
    type RunMode,
} from './disposition.js';

const ALL_CONTRACTS: GovernedContractId[] = [
    'data-snapshot',
    'edge-render-state',
    'privacy-export',
    'template-authoring-cem-ml',
    'token-outputs',
    'patch-transport',
];

const DATA_SECURITY: GovernedContractId[] = [
    'data-snapshot',
    'edge-render-state',
    'privacy-export',
];

const PRESENTATION: GovernedContractId[] = [
    'template-authoring-cem-ml',
    'token-outputs',
    'patch-transport',
];

const ALL_MODES: RunMode[] = ['application', 'build-ssr', 'development'];

describe('classifyContract (BR-VC-9)', () => {
    it('classes the data/security contracts strictly', () => {
        for (const c of DATA_SECURITY) {
            expect(classifyContract(c)).toBe<ContractClass>('data-security');
        }
    });

    it('classes the presentation contracts tolerantly', () => {
        for (const c of PRESENTATION) {
            expect(classifyContract(c)).toBe<ContractClass>('presentation');
        }
    });

    it('classifies every governed contract (exhaustive)', () => {
        for (const c of ALL_CONTRACTS) {
            const cls = classifyContract(c);
            expect(cls === 'data-security' || cls === 'presentation').toBe(true);
        }
        // The two partitions cover the whole domain with no overlap.
        expect(new Set([...DATA_SECURITY, ...PRESENTATION]).size).toBe(ALL_CONTRACTS.length);
    });
});

describe('decideDisposition — build/SSR is strict for all contracts (BR-VC-9)', () => {
    for (const c of ALL_CONTRACTS) {
        it(`rejects on ${c}`, () => {
            const d = decideDisposition('build-ssr', c);
            expect(d.strict).toBe(true);
            expect(d.disposition).toBe('reject');
            expect(d.mustUnderstand).toBe(false);
        });
    }
});

describe('decideDisposition — development is tolerant for all contracts (BR-VC-9)', () => {
    for (const c of ALL_CONTRACTS) {
        it(`degrades (does not reject) on ${c}`, () => {
            const d = decideDisposition('development', c);
            expect(d.strict).toBe(false);
            expect(d.disposition).toBe('degrade');
        });
    }
});

describe('decideDisposition — application is per-contract (BR-VC-9)', () => {
    for (const c of DATA_SECURITY) {
        it(`rejects on data/security contract ${c}`, () => {
            const d = decideDisposition('application', c);
            expect(d.strict).toBe(true);
            expect(d.disposition).toBe('reject');
            expect(d.contractClass).toBe('data-security');
        });
    }

    for (const c of PRESENTATION) {
        it(`tolerates on presentation contract ${c}`, () => {
            const d = decideDisposition('application', c);
            expect(d.strict).toBe(false);
            expect(d.disposition).toBe('degrade');
            expect(d.contractClass).toBe('presentation');
        });
    }
});

describe('decideDisposition — BR-VC-8 must-understand overrides in every mode', () => {
    for (const mode of ALL_MODES) {
        for (const c of ALL_CONTRACTS) {
            it(`rejects must-understand on ${c} in ${mode} mode`, () => {
                const d = decideDisposition(mode, c, { mustUnderstand: true });
                expect(d.disposition).toBe('reject');
                expect(d.strict).toBe(true);
                expect(d.mustUnderstand).toBe(true);
                expect(d.rationale).toContain('BR-VC-8');
            });
        }
    }

    it('does not flag mustUnderstand when the reject comes from the optional-feature policy', () => {
        const d = decideDisposition('build-ssr', 'token-outputs');
        expect(d.disposition).toBe('reject');
        expect(d.mustUnderstand).toBe(false);
    });
});

describe('decideDisposition — decision record is auditable', () => {
    it('echoes the active mode and contract and cites BR-VC-9', () => {
        const d = decideDisposition('application', 'data-snapshot');
        expect(d.mode).toBe('application');
        expect(d.contract).toBe('data-snapshot');
        expect(d.rationale).toContain('BR-VC-9');
    });
});

describe('ingestContractVersion — version negotiation feeds the disposition', () => {
    const BUILD = '1.2.0';

    it('accepts a version-less payload (BR-EV-5 expand-phase optional)', () => {
        const o = ingestContractVersion(undefined, BUILD, 'application', 'data-snapshot');
        expect(o).toEqual({ accept: true, reason: 'no-version' });
    });

    it('accepts an equal version as fully understood', () => {
        const o = ingestContractVersion('1.2.0', BUILD, 'application', 'data-snapshot');
        expect(o.accept).toBe(true);
        expect(o.reason).toBe('understood');
    });

    it('accepts a lower minor as fully understood', () => {
        const o = ingestContractVersion('1.0.5', BUILD, 'application', 'data-snapshot');
        expect(o.accept).toBe(true);
        expect(o.reason).toBe('understood');
    });

    it('rejects a malformed present version (cannot verify compatibility)', () => {
        const o = ingestContractVersion('1.x', BUILD, 'development', 'token-outputs');
        expect(o.accept).toBe(false);
        expect(o.reason).toBe('unparsable-version');
    });

    it('rejects a MAJOR mismatch as must-understand in every mode', () => {
        for (const mode of ALL_MODES) {
            const o = ingestContractVersion('2.0.0', BUILD, mode, 'token-outputs');
            expect(o.accept).toBe(false);
            expect(o.reason).toBe('incompatible-major');
            expect(o.decision?.mustUnderstand).toBe(true);
        }
    });

    describe('higher MINOR = unknown optional features → BR-VC-9 disposition', () => {
        it('application rejects on a data/security contract', () => {
            const o = ingestContractVersion('1.3.0', BUILD, 'application', 'data-snapshot');
            expect(o.reason).toBe('unknown-optional');
            expect(o.accept).toBe(false);
            expect(o.decision?.disposition).toBe('reject');
        });

        it('application tolerates on a presentation contract', () => {
            const o = ingestContractVersion('1.3.0', BUILD, 'application', 'token-outputs');
            expect(o.reason).toBe('unknown-optional');
            expect(o.accept).toBe(true);
            expect(o.decision?.disposition).toBe('degrade');
        });

        it('build/SSR rejects even a presentation contract', () => {
            const o = ingestContractVersion('1.3.0', BUILD, 'build-ssr', 'token-outputs');
            expect(o.accept).toBe(false);
            expect(o.decision?.disposition).toBe('reject');
        });

        it('development tolerates even a data/security contract', () => {
            const o = ingestContractVersion('1.3.0', BUILD, 'development', 'data-snapshot');
            expect(o.accept).toBe(true);
            expect(o.decision?.disposition).toBe('degrade');
        });
    });
});
