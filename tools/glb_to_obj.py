#!/usr/bin/env python3
"""Convert the supplied simple GLB mesh to OBJ for the course OBJ loader.

This utility supports the glTF 2.0 binary container and float POSITION,
NORMAL and TEXCOORD_0 accessors used by source_esfera.glb. It intentionally
keeps the conversion small and transparent for coursework inspection.
"""

import json
import struct
import sys
from pathlib import Path


def read_glb_mesh(path: Path):
    data = path.read_bytes()
    magic, version, _total_length = struct.unpack_from("<4sII", data, 0)
    if magic != b"glTF" or version != 2:
        raise ValueError("Expected a glTF 2.0 binary file (.glb)")

    json_length, json_type = struct.unpack_from("<II", data, 12)
    if json_type != 0x4E4F534A:
        raise ValueError("The first GLB chunk is not JSON")
    document = json.loads(data[20:20 + json_length].decode("utf-8"))

    binary_header = 20 + json_length
    binary_length, binary_type = struct.unpack_from("<II", data, binary_header)
    if binary_type != 0x004E4942:
        raise ValueError("The second GLB chunk is not BIN")
    binary = data[binary_header + 8:binary_header + 8 + binary_length]

    primitive = document["meshes"][0]["primitives"][0]

    def accessor_values(accessor_index: int, components: int):
        accessor = document["accessors"][accessor_index]
        if accessor["componentType"] != 5126:
            raise ValueError("Only float accessors are supported by this converter")
        view = document["bufferViews"][accessor["bufferView"]]
        offset = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        stride = view.get("byteStride", components * 4)
        values = []
        for index in range(accessor["count"]):
            values.append(struct.unpack_from("<" + "f" * components, binary, offset + index * stride))
        return values

    attributes = primitive["attributes"]
    return (
        accessor_values(attributes["POSITION"], 3),
        accessor_values(attributes["NORMAL"], 3),
        accessor_values(attributes["TEXCOORD_0"], 2),
    )


def convert(source: Path, destination: Path):
    positions, normals, uvs = read_glb_mesh(source)
    if not (len(positions) == len(normals) == len(uvs)):
        raise ValueError("POSITION, NORMAL and TEXCOORD_0 counts must match")
    if len(positions) % 3 != 0:
        raise ValueError("This simple converter expects a non-indexed triangle list")

    destination.parent.mkdir(parents=True, exist_ok=True)
    material_path = destination.with_suffix(".mtl")
    with destination.open("w", encoding="utf-8") as obj:
        obj.write("# Converted from the supplied GLB.\n")
        obj.write(f"mtllib {material_path.name}\no ImportedMesh\n")
        for x, y, z in positions:
            obj.write(f"v {x:.8f} {y:.8f} {z:.8f}\n")
        for u, v in uvs:
            obj.write(f"vt {u:.8f} {v:.8f}\n")
        for x, y, z in normals:
            obj.write(f"vn {x:.8f} {y:.8f} {z:.8f}\n")
        obj.write("usemtl ImportedMaterial\n")
        for first in range(0, len(positions), 3):
            a, b, c = first + 1, first + 2, first + 3
            obj.write(f"f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}\n")

    material_path.write_text(
        "newmtl ImportedMaterial\nKd 0.7991 0.7991 0.7991\nKs 0.5 0.5 0.5\nNs 64.0\n",
        encoding="utf-8",
    )
    print(f"Wrote {destination} ({len(positions)} vertices, {len(positions) // 3} triangles)")


if __name__ == "__main__":
    source_file = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("assets/models/source_esfera.glb")
    output_file = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("assets/models/esfera.obj")
    convert(source_file, output_file)
