import type { PopupRect } from './composite-navigation.js';
import type { NativeCemValue } from './native-values.js';

export type CemInvocationInputKind = 'pointer' | 'keyboard' | 'programmatic';
export interface CemInvocationGeometry {
    readonly pointer?: Readonly<PopupRect>;
    readonly selection?: Readonly<PopupRect>;
}
/** Transient browser/session references; native context stays in its retained channel. */
export interface CemSurfaceInvocation {
    readonly source?: HTMLElement;
    readonly command?: string | null;
    readonly contextKey?: string | null;
    readonly context?: NativeCemValue;
    readonly inputKind?: CemInvocationInputKind;
    readonly geometry?: CemInvocationGeometry;
    readonly returnDestination?: HTMLElement;
    readonly logicalParent?: HTMLElement;
}
export function snapshotGeometryRect(rect?: Readonly<PopupRect>, allowPoint = false): Readonly<PopupRect> | undefined {
    if (!rect || ![rect.left, rect.top, rect.right, rect.bottom].every(Number.isFinite)
        || (allowPoint ? rect.right < rect.left || rect.bottom < rect.top : rect.right <= rect.left || rect.bottom <= rect.top)) return;
    return Object.freeze({ left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom });
}
export function snapshotInvocationGeometry(kind: CemInvocationInputKind, point?: { x: number; y: number }, owner?: Readonly<PopupRect>, selection?: Readonly<PopupRect>): CemInvocationGeometry {
    const pointer = kind === 'keyboard' ? snapshotGeometryRect(owner)
        : kind === 'pointer' && point ? snapshotGeometryRect({ left: point.x, right: point.x, top: point.y, bottom: point.y }, true) : undefined;
    return Object.freeze({ pointer, selection: snapshotGeometryRect(selection) });
}
/** Capture now, before an ancestor hides; selection requires this explicit owner. */
export function captureCemSurfaceInvocation(source: HTMLElement, event?: Event, options: {
    command?: string | null; contextKey?: string | null; selectionOwner?: HTMLElement;
} = {}): CemSurfaceInvocation {
    const mouse = event instanceof MouseEvent ? event : undefined;
    const kind: CemInvocationInputKind = mouse?.detail ? 'pointer' : event instanceof KeyboardEvent || mouse ? 'keyboard' : 'programmatic';
    const selectionOwner = options.selectionOwner ?? source;
    const selection = source.ownerDocument.getSelection();
    const range = selection?.rangeCount ? selection.getRangeAt(0) : undefined;
    const selectionRect = range && !range.collapsed && selectionOwner.isConnected
        && selectionOwner.ownerDocument === source.ownerDocument && selectionOwner.contains(range.commonAncestorContainer)
        ? range.getBoundingClientRect() : undefined;
    return Object.freeze({ source, command: options.command ?? null, contextKey: options.contextKey ?? null, inputKind: kind,
        geometry: snapshotInvocationGeometry(kind, mouse ? { x: mouse.clientX, y: mouse.clientY } : undefined, source.getBoundingClientRect(), selectionRect) });
}
