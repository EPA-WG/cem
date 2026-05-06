import type { CemDiagnostic, CemDomFailLevel } from './cem-dom.ts';

export function isCemDomFailLevel(value: string): value is CemDomFailLevel {
    return value === 'parse' || value === 'validate' || value === 'strict';
}

export function hasFailingDiagnostics(
    diagnostics: readonly CemDiagnostic[],
    failLevel: CemDomFailLevel,
): boolean {
    return diagnostics.some((diagnostic) => isFailingDiagnostic(diagnostic, failLevel));
}

export function hasHardViolations(diagnostics: readonly CemDiagnostic[]): boolean {
    return diagnostics.some((diagnostic) => diagnostic.severity === 'error' || diagnostic.severity === 'fatal');
}

function isFailingDiagnostic(diagnostic: CemDiagnostic, failLevel: CemDomFailLevel): boolean {
    switch (failLevel) {
        case 'parse':
            return diagnostic.severity === 'fatal';
        case 'validate':
            return diagnostic.severity === 'error' || diagnostic.severity === 'fatal';
        case 'strict':
            return (
                diagnostic.severity === 'warning' ||
                diagnostic.severity === 'error' ||
                diagnostic.severity === 'fatal'
            );
    }
}
