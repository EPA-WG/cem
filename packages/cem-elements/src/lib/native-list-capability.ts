import type { CemProducedElementBehavior, SerializedPayloadNode } from './cem-elements.js';

/** A native, non-form-associated listbox over inert direct option payload. */
export const CEM_NATIVE_LIST_CAPABILITY: CemProducedElementBehavior = {
    beforeRender(instance, context) {
        const snapshot = context.snapshot();
        const choices = snapshot.payload.nodes.filter(
            (node): node is Extract<SerializedPayloadNode, { kind: 'element' }> =>
                node.kind === 'element' && node.tag === 'cem-list-option' && Object.hasOwn(node.attributes, 'value'),
        );
        const requested = snapshot.slices.value ?? instance.getAttribute('value');
        let authored = -1;
        choices.forEach((node, index) => { if (Object.hasOwn(node.attributes, 'selected')) authored = index; });
        const selected = requested != null
            ? choices.findIndex(node => node.attributes.value === String(requested))
            : authored >= 0 ? authored : choices.findIndex(node => !Object.hasOwn(node.attributes, 'disabled'));
        context.setSlices({ options: choices.map((node, index) => ({
            value: node.attributes.value,
            label: payloadText(node.children),
            disabled: Object.hasOwn(node.attributes, 'disabled'),
            selected: index === selected,
        })) }, { render: false });
    },
};

function payloadText(nodes: readonly SerializedPayloadNode[]): string {
    return nodes.map(node => node.kind === 'text' ? node.text : node.kind === 'element' ? payloadText(node.children) : '').join('');
}
