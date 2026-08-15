import { readFileSync, writeFileSync } from 'node:fs';
import { relative } from 'node:path';
import { deflateRawSync } from 'node:zlib';

import { listFiles } from './lib.mjs';

const localHeaderSignature = 0x04034b50;
const centralHeaderSignature = 0x02014b50;
const endSignature = 0x06054b50;
const utf8Flag = 0x0800;
const deflateMethod = 8;

export function writeDeterministicZip(destination, sourceRoot, rootName, epoch) {
    const entries = listFiles(sourceRoot).map((path) => ({
        name: `${rootName}/${relative(sourceRoot, path).replaceAll('\\', '/')}`,
        path,
    }));
    const { date, time } = dosTimestamp(epoch);
    const localParts = [];
    const centralParts = [];
    let offset = 0;

    for (const entry of entries) {
        const name = Buffer.from(entry.name, 'utf8');
        const data = readFileSync(entry.path);
        const compressed = deflateRawSync(data, { level: 9 });
        const checksum = crc32(data);
        const local = Buffer.alloc(30);
        local.writeUInt32LE(localHeaderSignature, 0);
        local.writeUInt16LE(20, 4);
        local.writeUInt16LE(utf8Flag, 6);
        local.writeUInt16LE(deflateMethod, 8);
        local.writeUInt16LE(time, 10);
        local.writeUInt16LE(date, 12);
        local.writeUInt32LE(checksum, 14);
        local.writeUInt32LE(compressed.byteLength, 18);
        local.writeUInt32LE(data.byteLength, 22);
        local.writeUInt16LE(name.byteLength, 26);
        local.writeUInt16LE(0, 28);
        localParts.push(local, name, compressed);

        const central = Buffer.alloc(46);
        central.writeUInt32LE(centralHeaderSignature, 0);
        central.writeUInt16LE(0x0314, 4);
        central.writeUInt16LE(20, 6);
        central.writeUInt16LE(utf8Flag, 8);
        central.writeUInt16LE(deflateMethod, 10);
        central.writeUInt16LE(time, 12);
        central.writeUInt16LE(date, 14);
        central.writeUInt32LE(checksum, 16);
        central.writeUInt32LE(compressed.byteLength, 20);
        central.writeUInt32LE(data.byteLength, 24);
        central.writeUInt16LE(name.byteLength, 28);
        central.writeUInt16LE(0, 30);
        central.writeUInt16LE(0, 32);
        central.writeUInt16LE(0, 34);
        central.writeUInt16LE(0, 36);
        central.writeUInt32LE((0o100644 << 16) >>> 0, 38);
        central.writeUInt32LE(offset, 42);
        centralParts.push(central, name);
        offset += local.byteLength + name.byteLength + compressed.byteLength;
    }

    const centralDirectory = Buffer.concat(centralParts);
    const end = Buffer.alloc(22);
    end.writeUInt32LE(endSignature, 0);
    end.writeUInt16LE(0, 4);
    end.writeUInt16LE(0, 6);
    end.writeUInt16LE(entries.length, 8);
    end.writeUInt16LE(entries.length, 10);
    end.writeUInt32LE(centralDirectory.byteLength, 12);
    end.writeUInt32LE(offset, 16);
    end.writeUInt16LE(0, 20);
    writeFileSync(destination, Buffer.concat([...localParts, centralDirectory, end]));
}

function dosTimestamp(epoch) {
    const value = new Date(epoch * 1000);
    const year = Math.max(1980, Math.min(2107, value.getUTCFullYear()));
    const date = ((year - 1980) << 9) | ((value.getUTCMonth() + 1) << 5) | value.getUTCDate();
    const time = (value.getUTCHours() << 11) | (value.getUTCMinutes() << 5) | (value.getUTCSeconds() >> 1);
    return { date, time };
}

function crc32(buffer) {
    let crc = 0xffffffff;
    for (const byte of buffer) {
        crc ^= byte;
        for (let bit = 0; bit < 8; bit += 1) {
            crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
        }
    }
    return (crc ^ 0xffffffff) >>> 0;
}
