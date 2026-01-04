import { cemTheme } from './cem-theme.js';
import packageJson from '../../package.json' with { type: 'json' };

describe('cemTheme', () => {
  it('should work', () => {
    expect(cemTheme()['@epa-wg/cem-theme']).toEqual(packageJson.version); // i.e. > '0.0.1'
  });
});
