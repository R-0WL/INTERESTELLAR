#!/usr/bin/env python3
"""Prepare the supplied GLB assets as compact, centered OBJ meshes.

Requires: trimesh, numpy, scipy. The game itself only needs the generated OBJ files.
"""
from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
import trimesh
from scipy.spatial import ConvexHull

ASSET_DIR = Path(__file__).resolve().parents[1] / "assets" / "models" / "objs"

# source GLB, output OBJ, voxel-cluster tolerance (None = no clustering), and
# optional convex-hull vertex budget for the frequently-instanced asteroid.
ASSETS = [
    ("blackhole.glb", "blackhole.obj", 0.08, None),
    ("endurance + ranger + ranger.glb", "endurance_ranger_ranger.obj", 0.60, None),
    ("endurance + ranger.glb", "endurance_ranger.obj", 0.60, None),
    ("ranger_ship_of_interstellar_free.glb", "ranger.obj", None, None),
    ("Asteroid.glb", "asteroid.obj", None, 96),
]


def scene_mesh(source: Path) -> tuple[np.ndarray, np.ndarray]:
    scene = trimesh.load(source, force="scene", process=False)
    vertices: list[np.ndarray] = []
    faces: list[np.ndarray] = []
    offset = 0
    for node in scene.graph.nodes_geometry:
        transform, geometry_name = scene.graph.get(node)
        geometry = scene.geometry[geometry_name]
        local = np.asarray(geometry.vertices, dtype=np.float64)
        world = (transform[:3, :3] @ local.T).T + transform[:3, 3]
        triangles = np.asarray(geometry.faces, dtype=np.int64)
        if not len(world) or not len(triangles):
            continue
        vertices.append(world)
        faces.append(triangles + offset)
        offset += len(world)
    if not vertices:
        raise ValueError(f"No triangle geometry in {source.name}")
    return np.vstack(vertices), np.vstack(faces)


def voxel_cluster(vertices: np.ndarray, faces: np.ndarray, cell: float) -> tuple[np.ndarray, np.ndarray]:
    keys = np.floor(vertices / cell).astype(np.int64)
    _, inverse = np.unique(keys, axis=0, return_inverse=True)
    clustered = np.zeros((int(inverse.max()) + 1, 3), dtype=np.float64)
    np.add.at(clustered, inverse, vertices)
    counts = np.bincount(inverse)
    clustered /= counts[:, None]

    remapped = inverse[faces]
    valid = (
        (remapped[:, 0] != remapped[:, 1])
        & (remapped[:, 1] != remapped[:, 2])
        & (remapped[:, 0] != remapped[:, 2])
    )
    remapped = remapped[valid]
    if len(remapped):
        canonical = np.sort(remapped, axis=1)
        _, first = np.unique(canonical, axis=0, return_index=True)
        remapped = remapped[np.sort(first)]
    return clustered, remapped


def asteroid_hull(vertices: np.ndarray, budget: int) -> tuple[np.ndarray, np.ndarray]:
    # Farthest-point sampling keeps the large-scale irregular silhouette rather
    # than retaining a random patch from the original high-density surface.
    center = vertices.mean(axis=0)
    first = int(np.argmax(np.sum((vertices - center) ** 2, axis=1)))
    selected = [first]
    min_dist2 = np.sum((vertices - vertices[first]) ** 2, axis=1)
    for _ in range(1, min(budget, len(vertices))):
        index = int(np.argmax(min_dist2))
        selected.append(index)
        dist2 = np.sum((vertices - vertices[index]) ** 2, axis=1)
        min_dist2 = np.minimum(min_dist2, dist2)
    points = vertices[np.asarray(selected)]
    hull = ConvexHull(points)
    return points, np.asarray(hull.simplices, dtype=np.int64)


def write_obj(path: Path, vertices: np.ndarray, faces: np.ndarray) -> None:
    # Normalize all models to a two-unit maximum dimension, centered at origin.
    lower = vertices.min(axis=0)
    upper = vertices.max(axis=0)
    center = (lower + upper) * 0.5
    max_extent = float(np.max(upper - lower))
    if max_extent <= 1e-9:
        raise ValueError(f"Degenerate geometry: {path.name}")
    normalized = (vertices - center) * (2.0 / max_extent)

    with path.open("w", encoding="utf-8", newline="\n") as output:
        output.write(f"# Generated from supplied GLB asset. Vertices: {len(normalized)}, triangles: {len(faces)}\n")
        for x, y, z in normalized:
            output.write(f"v {x:.6f} {y:.6f} {z:.6f}\n")
        for a, b, c in faces:
            output.write(f"f {int(a) + 1} {int(b) + 1} {int(c) + 1}\n")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", nargs="*", help="Output OBJ basenames to regenerate")
    args = parser.parse_args()
    allowed = set(args.only or [])
    for source_name, output_name, tolerance, hull_budget in ASSETS:
        if allowed and output_name not in allowed:
            continue
        source = ASSET_DIR / source_name
        target = ASSET_DIR / output_name
        vertices, faces = scene_mesh(source)
        if hull_budget is not None:
            vertices, faces = asteroid_hull(vertices, hull_budget)
        elif tolerance is not None:
            vertices, faces = voxel_cluster(vertices, faces, tolerance)
        write_obj(target, vertices, faces)
        print(f"{source_name} -> {output_name}: {len(vertices):,} vertices, {len(faces):,} triangles")


if __name__ == "__main__":
    main()
