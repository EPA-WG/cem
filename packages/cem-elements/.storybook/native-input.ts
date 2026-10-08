import type { CDPSession } from '@vitest/browser-playwright';

/** Chromium browser-input driver for acceptance plays; no synthetic DOM events. */
export function nativeInputPoint(element: Element) {
    const rect = element.getBoundingClientRect();
    let x = rect.left + rect.width / 2, y = rect.top + rect.height / 2;
    let view = element.ownerDocument.defaultView;
    while (view?.frameElement) {
        const frame = view.frameElement, bounds = frame.getBoundingClientRect();
        x = bounds.left + (x + frame.clientLeft) * bounds.width / (frame.clientWidth + 2 * frame.clientLeft);
        y = bounds.top + (y + frame.clientTop) * bounds.height / (frame.clientHeight + 2 * frame.clientTop);
        view = frame.ownerDocument.defaultView;
    }
    return { x, y };
}
export async function nativeTap(driver: CDPSession, element: Element, pointerType: 'mouse' | 'touch' | 'pen') {
    await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
    const point = nativeInputPoint(element);
    if (pointerType === 'touch') {
        await driver.send('Emulation.setTouchEmulationEnabled', { enabled: true });
        try {
            await driver.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ ...point, id: 1 }] });
            await driver.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
        } finally { await driver.send('Emulation.setTouchEmulationEnabled', { enabled: false }); }
    } else {
        await driver.send('Input.dispatchMouseEvent', { type: 'mouseMoved', ...point, button: 'none', buttons: 0, pointerType });
        await driver.send('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', buttons: 1, clickCount: 1, pointerType });
        await driver.send('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', buttons: 0, clickCount: 1, pointerType });
    }
}

/** Native touch cancellation/panning; movement remains browser-owned scrolling. */
export async function nativeTouchGesture(driver: CDPSession, element: Element, action: 'pan' | 'cancel') {
    await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
    const point = nativeInputPoint(element);
    await driver.send('Emulation.setTouchEmulationEnabled', { enabled: true });
    try {
        if (action === 'pan') {
            await driver.send('Input.synthesizeScrollGesture', { ...point, yDistance: -40, gestureSourceType: 'touch', preventFling: true, speed: 200 });
        } else {
            await driver.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ ...point, id: 1 }] });
            await driver.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
        }
        await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
    } finally {
        await driver.send('Emulation.setTouchEmulationEnabled', { enabled: false });
    }
}
export async function nativePenDrag(driver: CDPSession, element: Element) {
    const point = nativeInputPoint(element);
    await driver.send('Input.dispatchMouseEvent', { type: 'mouseMoved', ...point, pointerType: 'pen' });
    await driver.send('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', buttons: 1, clickCount: 1, pointerType: 'pen' });
    await driver.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: point.x + 30, y: point.y, buttons: 1, pointerType: 'pen' });
    await driver.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: point.x + 30, y: point.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'pen' });
}

/** Native wheel input proves the list remains scrollable without arming a row. */
export async function nativeWheel(driver: CDPSession, element: Element) {
    const point = nativeInputPoint(element);
    await driver.send('Input.dispatchMouseEvent', { type: 'mouseMoved', ...point, pointerType: 'mouse' });
    await driver.send('Input.dispatchMouseEvent', { type: 'mouseWheel', ...point, deltaX: 0, deltaY: 40 });
}
