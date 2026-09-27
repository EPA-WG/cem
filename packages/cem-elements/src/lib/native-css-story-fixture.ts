/** Browser acceptance adapter: install native output only inside test fixtures. */
import { createCemDeclarationScope } from './declaration-scope.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import { createCemProcessingTextSource } from './internal/runtime-support/processing-host.js';

export function nativeCssStoryHost(fallback = false) {
    const scope = createCemDeclarationScope({ document });
    const host = cemProcessingHostForScope(scope, {
        workerScriptUrl: new URL('./internal/runtime-support/processing-worker.js', import.meta.url),
        ...(fallback ? { workerFactory: () => { throw new Error('fixture requests main-thread fallback'); } } : {}),
    });
    return {
        host,
        dispose: () => scope.dispose(),
        compile: (tag: string, sources: Array<{ css: string; scope: string | null; contentType?: string }>) => host.compile({
            language: 'css', producedTag: tag, templateArtifactId: tag,
            registrationIdentity: `native-story:${tag}`,
            source: createCemProcessingTextSource(JSON.stringify(sources)),
            sourceRef: { kind: 'inline', value: tag }, resolverIdentity: 'native-story',
            scopePolicyStamp: 'native-story', sourceMapMode: 'dev',
        }).result,
    };
}

export const nativeCssStoryContext = {
    identity: 'native-story', resolverIdentity: 'native-story', resourcePolicyStamp: 'native-story', frames: [],
};
