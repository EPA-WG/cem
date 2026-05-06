import { runCemDomCli } from '../src/cli.ts';

const result = await runCemDomCli(['fixture', 'validate'], {
    cwd: process.cwd(),
});

if (result.stdout) {
    process.stdout.write(result.stdout);
}

if (result.stderr) {
    process.stderr.write(result.stderr);
}

process.exitCode = result.exitCode;
