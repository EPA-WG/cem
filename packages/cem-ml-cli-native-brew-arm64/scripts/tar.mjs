import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { relative } from 'node:path';
import { gzipSync } from 'node:zlib';

import { listFiles } from './lib.mjs';

const blockSize = 512;

export function writeDeterministicTarGz(destination, sourceRoot, rootName, epoch) {
    const entries = [directoryEntry(rootName)];
    const directories = new Set();
    for (const path of listFiles(sourceRoot)) {
        const relativePath = relative(sourceRoot, path).replaceAll('\\', '/');
        const parts = relativePath.split('/');
        for (let index = 1; index < parts.length; index += 1) {
            directories.add(`${rootName}/${parts.slice(0, index).join('/')}`);
        }
        entries.push(fileEntry(`${rootName}/${relativePath}`, path));
    }
    entries.push(...[...directories].sort().map(directoryEntry));
    entries.sort((left, right) => left.name.localeCompare(right.name));

    const blocks = [];
    for (const entry of entries) {
        const data = entry.path === undefined ? Buffer.alloc(0) : readFileSync(entry.path);
        blocks.push(tarHeader(entry.name, entry.mode, data.byteLength, epoch, entry.type));
        if (data.byteLength > 0) {
            blocks.push(data);
            const remainder = data.byteLength % blockSize;
            if (remainder !== 0) blocks.push(Buffer.alloc(blockSize - remainder));
        }
    }
    blocks.push(Buffer.alloc(blockSize * 2));
    writeFileSync(destination, gzipSync(Buffer.concat(blocks), { level: 9, mtime: 0 }));
}

function directoryEntry(name) {
    return { name: `${name.replace(/\/$/, '')}/`, mode: 0o755, type: '5' };
}

function fileEntry(name, path) {
    return { name, path, mode: statSync(path).mode & 0o777, type: '0' };
}

function tarHeader(name, mode, size, epoch, type) {
    if (Buffer.byteLength(name) > 100) throw new Error(`tar path exceeds ustar name field: ${name}`);
    const header = Buffer.alloc(blockSize);
    writeText(header, 0, 100, name);
    writeOctal(header, 100, 8, mode);
    writeOctal(header, 108, 8, 0);
    writeOctal(header, 116, 8, 0);
    writeOctal(header, 124, 12, size);
    writeOctal(header, 136, 12, epoch);
    header.fill(0x20, 148, 156);
    writeText(header, 156, 1, type);
    writeText(header, 257, 6, 'ustar\0');
    writeText(header, 263, 2, '00');
    writeText(header, 265, 32, 'root');
    writeText(header, 297, 32, 'wheel');
    const checksum = header.reduce((sum, byte) => sum + byte, 0);
    const encoded = checksum.toString(8).padStart(6, '0');
    writeText(header, 148, 6, encoded);
    header[154] = 0;
    header[155] = 0x20;
    return header;
}

function writeOctal(buffer, offset, length, value) {
    const encoded = value.toString(8).padStart(length - 1, '0');
    if (encoded.length >= length) throw new Error(`tar numeric field overflow: ${value}`);
    writeText(buffer, offset, length - 1, encoded);
    buffer[offset + length - 1] = 0;
}

function writeText(buffer, offset, length, value) {
    const written = buffer.write(value, offset, length, 'utf8');
    if (written !== Buffer.byteLength(value)) throw new Error(`tar field overflow: ${value}`);
}
