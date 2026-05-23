#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import Ajv2020 from 'ajv/dist/2020.js';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, '../..');
const schemaPath = path.join(
  workspaceRoot,
  'packages/cem_ml/schema/observability/report-event.schema.json',
);

const schema = JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
const ajv = new Ajv2020({ allErrors: true, strict: false });
const validate = ajv.compile(schema);

const validCases = [
  {
    name: 'parse event with nullable Option<String> fields',
    event: {
      sequence: 0,
      channel: 'parse',
      byteOffset: 0,
      parse: { kind: 'trivia', name: null, value: null },
    },
  },
  {
    name: 'validate event with multi-segment diagnostic code',
    event: {
      sequence: 1,
      channel: 'validate',
      byteOffset: 9,
      validate: {
        code: 'cem.schema.scoping.exclusive_src_select',
        severity: 'error',
        message: 'selectors are mutually exclusive',
      },
    },
  },
  {
    name: 'validate event with digit-bearing diagnostic code',
    event: {
      sequence: 2,
      channel: 'validate',
      validate: {
        code: 'cem.a11y.accessible_name_missing',
        severity: 'warning',
        message: 'missing accessible name',
      },
    },
  },
  {
    name: 'transform event with unit TransformKind object',
    event: {
      sequence: 3,
      channel: 'transform',
      transform: {
        transform: { kind: 'CemTokenizer' },
        summary: 'tokenized',
      },
    },
  },
  {
    name: 'transform event with SchemaValidation payload',
    event: {
      sequence: 4,
      channel: 'transform',
      transform: {
        transform: { kind: 'SchemaValidation', schema_id: 7 },
        summary: 'validated',
      },
    },
  },
  {
    name: 'transform event with HandoffBoundary payload',
    event: {
      sequence: 5,
      channel: 'transform',
      transform: {
        transform: {
          kind: 'HandoffBoundary',
          child_content_type: 'text/html',
        },
        summary: 'entered child content',
      },
    },
  },
  {
    name: 'transform event with ContentTypeTransform payload',
    event: {
      sequence: 6,
      channel: 'transform',
      transform: {
        transform: {
          kind: 'ContentTypeTransform',
          content_type: 'application/cem+xml',
        },
        summary: 'converted content type',
      },
    },
  },
];

const invalidCases = [
  {
    name: 'parse event cannot carry validate payload',
    event: {
      sequence: 10,
      channel: 'parse',
      byteOffset: 0,
      parse: { kind: 'name', name: 'id', value: null },
      validate: {
        code: 'cem.test.invalid',
        severity: 'error',
        message: 'mixed payload',
      },
    },
  },
  {
    name: 'validate event cannot carry transform payload',
    event: {
      sequence: 11,
      channel: 'validate',
      validate: {
        code: 'cem.test.invalid',
        severity: 'error',
        message: 'mixed payload',
      },
      transform: {
        transform: { kind: 'CemTokenizer' },
        summary: 'mixed payload',
      },
    },
  },
  {
    name: 'transform event cannot carry parse payload',
    event: {
      sequence: 12,
      channel: 'transform',
      parse: { kind: 'name', name: 'id', value: null },
      transform: {
        transform: { kind: 'CemTokenizer' },
        summary: 'mixed payload',
      },
    },
  },
  {
    name: 'parse event requires byteOffset',
    event: {
      sequence: 13,
      channel: 'parse',
      parse: { kind: 'name', name: 'id', value: null },
    },
  },
  {
    name: 'validate code must be a lowercase cem dotted path',
    event: {
      sequence: 14,
      channel: 'validate',
      validate: {
        code: 'cem.Bad.code',
        severity: 'error',
        message: 'bad code',
      },
    },
  },
  {
    name: 'transform kind cannot serialize as a bare string',
    event: {
      sequence: 15,
      channel: 'transform',
      transform: {
        transform: 'CemTokenizer',
        summary: 'old shape',
      },
    },
  },
  {
    name: 'SchemaValidation transform requires schema_id',
    event: {
      sequence: 16,
      channel: 'transform',
      transform: {
        transform: { kind: 'SchemaValidation' },
        summary: 'missing schema id',
      },
    },
  },
  {
    name: 'unit transform kinds cannot carry variant payload fields',
    event: {
      sequence: 17,
      channel: 'transform',
      transform: {
        transform: { kind: 'CemTokenizer', schema_id: 1 },
        summary: 'extra field',
      },
    },
  },
];

let failures = 0;

for (const { name, event } of validCases) {
  if (!validate(event)) {
    failures += 1;
    console.error(`Expected valid case to pass: ${name}`);
    console.error(JSON.stringify(validate.errors, null, 2));
  }
}

for (const { name, event } of invalidCases) {
  if (validate(event)) {
    failures += 1;
    console.error(`Expected invalid case to fail: ${name}`);
  }
}

if (failures > 0) {
  process.exitCode = 1;
} else {
  console.log(
    `Validated ${validCases.length + invalidCases.length} observability schema contract cases.`,
  );
}
