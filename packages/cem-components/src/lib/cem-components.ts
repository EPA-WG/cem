import { cemTheme } from '@epa-wg/cem-theme';

export function cemComponents(): string {
  return 'cem-components';
}

export function getTheme(): string {
  return cemTheme();
}
