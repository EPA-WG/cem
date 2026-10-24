export * from './lib/cem-elements.js';
export * from './lib/repository.js';
export { openCemHttpResource } from './lib/http-resource-transport.js';
export { CemSchemaAssetPackLoader, CemSchemaAssetLoadError, type CemSchemaAssetPackOptions, type CemSchemaAssetRuntime,
    type CemSchemaAssetNativeRuntime, type CemSchemaAssetSourceIdentity, type CemSchemaAssetPackageIdentity, type CemLoadedSchemaAssetSource } from './lib/schema-asset-pack.js';
export { CemCommittedCheckpointStore, type CemCommitReceipt, type CemCheckpointNativeRuntime, type CemCheckpointOwnerReceipt, type CemCheckpointSealer, type CemStoredCheckpoint, type CemCheckpointRetirementHandoff } from './lib/committed-checkpoint.js';
export { mountCemView, type CemHostCommandRequest, type CemHostViewOptions, type CemHostView } from './lib/host-view.js';
export {
    CemDeclarationScopeError,
    createCemDeclarationScope,
    getDefaultCemDeclarationScope,
    type CemDeclarationScope,
    type CemDeclarationScopeErrorCode,
    type CemDeclarationScopeOptions,
} from './lib/declaration-scope.js';
export * from './lib/legacy-xslt/contract.js';
// The CEM-owned legacy HTML+XSLT compiler (cem_ml engine via the cem_ql WASM module), shared by the
// browser runtime, SSR, and fixture gates.
export { convertLegacyTemplate, type LegacyConvertResult } from './lib/internal/runtime-support/cem-ql-render.js';

export { exportNativeCemAttributes, importNativeCemAttributes, exportNativeCemSlices, importNativeCemSlices,
    type NativeCemValue, type NativeCemAttributeBinding, type NativeCemSliceBinding, type CemValueArtifactLimits,
} from './lib/native-values.js';

export { publishEdgeCssDomUpdate, type CemEdgeCssPublicationOptions } from './lib/edge-css-publication.js';
export { DeclarationStyleOwnership } from './lib/declaration-style-ownership.js';
export type { CemEdgeStylesheetBatch, CemEdgeStylesheetState, CemEdgeSsrRenderUpdateResult } from './lib/edge-ssr-host.js';

export { getCemActionInvocation, type CemActionInvocation } from './lib/action-command-capability.js';
export { connectCemNativeSurface, type CemNativeSurfaceController, type CemNativeSurfaceKind, type CemNativeSurfaceOptions, type CemSurfacePreparation } from './lib/native-surface.js';
export { captureCemSurfaceInvocation, type CemSurfaceInvocation, type CemInvocationGeometry, type CemInvocationInputKind } from './lib/surface-invocation.js';

export type { CemElementReferenceInputs } from './lib/element-reference-inputs.js';
export { DEFAULT_ARIA_REFERENCE_PROFILE, EXPERIMENTAL_ARIA_REFERENCE_PROFILE, resolveAriaReferenceProfile, type CemAriaReferenceProfile } from './lib/aria-reference-profile.js';
export { CemElementPlacementCoordinator, type CemBrowserPlacementStage, type CemBrowserPlacementPublication, type CemBrowserPlacementRegistration,
    type CemPlacementLease, type CemPlacementTransaction } from './lib/element-placement-coordinator.js';
export type { CemElementPlacementUse, CemElementPlacementSnapshot } from './lib/element-reference-inputs.js';
export { CemSsrPlacementCoordinator, type CemSsrPlacementStage, type CemSsrPlacementPublication, type CemPlacementResumeHints } from './lib/element-placement-ssr.js';

export { createCemPlatformCloseCoordinator, type CemPlatformCloseCoordinator, type CemPlatformCloseAuthority, type CemPlatformCloseContext, type CemPlatformCloseLease } from './lib/platform-close.js';

export type { CemTabDetail } from './lib/tab-panels-capability.js';
export type { CemStepDetail } from './lib/workflow-panels-capability.js';
export type { CemTreeActivateDetail, CemTreeToggleDetail } from './lib/tree-view-capability.js';

export type { CemSortDirection, CemSortDetail } from './lib/sort-action-capability.js';
export type { CemPageDetail } from './lib/pagination-capability.js';

export { mountCemLivePreview, type CemLivePreview, type CemLivePreviewOptions, type CemLivePreviewResource, type CemLivePreviewResources } from './lib/live-preview.js';
export { registerCemArtifactTree, mountCemArtifactPreview, type CemPreviewArtifact, type CemArtifactTree, type CemArtifactPreview, type CemArtifactPreviewOptions } from './lib/artifact-preview.js';

export { CemCheckpointPublicationCoordinator, type CemCheckpointPublicationOptions, type CemCheckpointEndpointOwner, type CemCheckpointPublicationOwner, type CemCheckpointJournalAuthority, type CemCheckpointOperation } from './lib/checkpoint-publication.js';

export type { CemCheckpointRelationship } from './lib/committed-checkpoint.js';

export type { CemNativeDrain, CemRetiredRenderingGeneration } from './lib/internal/runtime-support/checkpoint-domain.js';

export { isolateCemFallbackGeneration, type CemOwnedFallbackGeneration, type CemFallbackWasmOwnership, type CemFallbackRetirementConsent, type CemDisposedFallbackGeneration } from './lib/internal/runtime-support/native-runtime.js';

export { CemNativeRecoveryScope, type CemNativeRecoveryEnrollment, type CemNativeRecoveryEndpoint, CemNativeGenerationGroup, type CemNativeGenerationParticipant, type CemNativeGenerationAdmission, type CemNativeGenerationBindingTicket } from './lib/internal/runtime-support/native-generation.js';
export type { CemFreshFallbackGeneration, CemFallbackNativeEntrypoint, CemFallbackNativeCleanup } from './lib/internal/runtime-support/native-runtime.js';
export type { CemFreshRenderingGeneration } from './lib/internal/runtime-support/processing-host-runtime.js';

export { prepareCemCheckpointProcessing, type CemCheckpointProcessingOptions, type CemResumedProcessingOwner } from './lib/checkpoint-processing.js';
export { resumeCemCheckpointOwners, type CemCheckpointResumeAdapter, type CemCheckpointResumeContext, type CemCheckpointResumeTicket, type CemPreparedCheckpointOwner, type CemResumedCheckpointOwners } from './lib/checkpoint-resume.js';

export type { CemResumedElementOwner } from './lib/cem-elements.js';

export { CemRecoveryStatusObserver, type CemRecoveryStatus, type CemRecoveryStatusState, type CemRecoveryStatusReason, type CemRecoveryStatusOptions } from './lib/recovery-status.js';
