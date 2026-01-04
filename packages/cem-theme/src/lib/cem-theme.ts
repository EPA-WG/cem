import packageJson from '../../package.json' with { type: 'json' };

export function cemTheme(): Record<string, string> {
  return {"@epa-wg/cem-theme": packageJson.version, type: packageJson.type, versionBump: '0.0.4b'};
}
