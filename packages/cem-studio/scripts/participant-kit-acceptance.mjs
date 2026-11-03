import assert from 'node:assert/strict';
import { presenterPrivacy } from './presenter-project.mjs';

/** Validate an exercise report against the specific artifact and host being claimed.
 * This checks recorded evidence; it does not rerun a browser or authenticate a release.
 */
export function verifyParticipantAcceptance(report, expected) {
    assert.equal(report.schemaVersion, 1);
    for (const field of ['archiveSha256', 'sourceRevision', 'frozenSourceRevision', 'frozenSupplierRevision']) {
        assert.match(report[field] ?? '', field === 'archiveSha256' ? /^[a-f0-9]{64}$/ : /^[a-f0-9]{40}$/, `Missing identity: ${field}`);
    }
    for (const field of ['archiveSha256', 'sourceRevision', 'node', 'platform', 'arch', 'browser']) {
        assert.ok(typeof expected[field] === 'string' && expected[field].length, `Missing claim: ${field}`);
        assert.equal(report[field], expected[field], `Acceptance belongs to another artifact/platform: ${field}`);
    }
    for (const field of ['extractedOutsideWorkspace', 'originalProductionIndex', 'offline', 'draftPreservation', 'independentCopies']) {
        assert.equal(report[field], true, `Missing exercise evidence: ${field}`);
    }
    assert.equal(report.launcher, 'node-builtins');
    assert.equal(report.installedApplicationDependencies, false);
    assert.ok(Number.isInteger(report.fileCount) && report.fileCount > 0);
    assert.equal(report.loginConfigurations, 20, 'Incomplete Login exercise');
    assert.equal(report.supplierConfigurations, 3, 'Incomplete Supplier exercise');
    assert.deepEqual(report.advertisedAmsterdamRoutes, ['login', 'supplier-pipeline']);
    assert.deepEqual(report.credentialPolicy, presenterPrivacy, 'Credential policy changed');
    assert.equal(report.publicAvailability, 'unverified', 'Local kit cannot establish public availability');
    return { route: 'local-transfer', archiveSha256: report.archiveSha256,
        node: report.node, platform: report.platform, arch: report.arch, browser: report.browser,
        publicAvailability: 'unverified', freshInstallation: 'unverified', otherPlatforms: 'unverified' };
}
