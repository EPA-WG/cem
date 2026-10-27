import { beforeAll } from 'vitest';
import { server } from 'vitest/browser';

beforeAll(() => server.commands.startCemBrowserLoadDiagnostics());
