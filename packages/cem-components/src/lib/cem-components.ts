import { cemTheme } from '@epa-wg/cem-theme';

export function cemComponents(): string {
  return '@epa-wg/cem-components';
}

export function getTheme(): string {
  return cemTheme()['@epa-wg/cem-theme'];
}
