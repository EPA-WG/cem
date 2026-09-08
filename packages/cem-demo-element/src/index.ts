export * from './cem-demo-element.js';
export * from './cem-ml-runtime.js';

import { defineCemDemoElement } from './cem-demo-element.js';

if (typeof customElements !== 'undefined') defineCemDemoElement();
