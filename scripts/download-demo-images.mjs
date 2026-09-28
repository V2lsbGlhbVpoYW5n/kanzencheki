import { createHash } from "node:crypto";
import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = process.argv[2] ? resolve(process.argv[2]) : join(root, "static/demo");
const images = [
  {
    name: "signed.jpg",
    url: "https://assets.st-note.com/img/1738132277-uLNlqdB8I2vJo9wZSeChUmGW.jpg",
    sha256: "1c466cf01107efe439837774a6a9581194edd103da85282a998976298330194d",
  },
  {
    name: "pair.jpg",
    url: "https://assets.st-note.com/img/1748262745-1eCRgcU5svBMimAbHTSdpQI6.jpg?width=1000",
    sha256: "28383e08c9b2be7a225622646daf3228e4cfc2151003bdf3323d8e844eae6700",
  },
  {
    name: "collection.jpg",
    url: "https://booth.pximg.net/66098a95-9e20-4a5d-8515-a4e4ff60cef6/i/5802885/973b1fe3-3d4a-44de-841e-fce9ede1e295_base_resized.jpg",
    sha256: "c6093c4bc399c8877a073ae0c38b5b16d82362d01137ecbca0ac0d753297596d",
  },
  {
    name: "birthday.jpg",
    url: "https://images.groobee.com/images/asobisystem/YydGgeieH1o49uS0VaNo3xno27rdX3bj80fiKv9z.jpeg",
    sha256: "f9340eef32a0d1eac83a07809524f9b73da499b2bbb73a4874e3a16a04a9a464",
  },
  {
    name: "cafe.png",
    url: "https://www.codigonuevo.com/binrepository/cn-articulo-instax-241125-web_335-73990931_20241125160356.png",
    sha256: "d404b92e403234e64bfffa3f65480a6479d01ecb3f52c2374b469c575ec77808",
  },
];

function digest(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

await mkdir(output, { recursive: true });
let failed = false;
for (const { name, url, sha256 } of images) {
  const destination = join(output, name);
  try {
    if (digest(await readFile(destination)) === sha256) {
      console.log(`${name}: already present`);
      continue;
    }
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }

  const temporary = join(output, `.${name}.${process.pid}.tmp`);
  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(30_000) });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    if (digest(bytes) !== sha256) {
      throw new Error("source image changed (SHA-256 mismatch)");
    }
    await writeFile(temporary, bytes);
    await rename(temporary, destination);
    console.log(`${name}: downloaded`);
  } catch (error) {
    failed = true;
    console.error(`${name}: ${error.message}`);
    await rm(temporary, { force: true });
  }
}
if (failed) process.exitCode = 1;
