import { cemTheme } from './cem-theme.js';

describe('cemTheme', () => {
  it('should work', () => {
    expect(cemTheme()['@epa-wg/cem-theme']).toEqual('0.0.0');
  });
});
