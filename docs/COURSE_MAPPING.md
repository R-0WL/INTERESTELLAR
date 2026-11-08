# Mapa de conceptos del curso

Este documento conecta las etapas identificadas en `cc2018-2026-02-10` con los módulos de esta adaptación.

## Software Rendering

- **Framebuffer:** `framebuffer.rs` administra el arreglo de píxeles y el Z-buffer.
- **Puntos y líneas:** `set_pixel` y `draw_line`.
- **Polígonos:** las mallas se almacenan como arreglos de triángulos indexados.
- **Relleno:** `rasterize_triangle` recorre el bounding box y calcula pesos baricéntricos.
- **Ventana y render loop:** `main.rs` usa `minifb` para entrada y presentación.

## Render Pipeline

- **Modelo:** cada objeto aplica traslación, rotación y escala.
- **Vista:** `Mat4::look_at` transforma el mundo al espacio de cámara.
- **Proyección:** `Mat4::perspective` genera coordenadas homogéneas.
- **Viewport:** el rasterizador transforma NDC a coordenadas de pantalla.
- **Modelos:** `load_obj` lee posiciones, UV, normales y caras.
- **Interpolación:** `rasterize_triangle` interpola profundidad y atributos con corrección de perspectiva.
- **Iluminación:** `fragment` combina luz ambiental, difusa y especular.
- **Fragment shader:** `sample_material` genera patrones procedurales por tipo de cuerpo.

## Decisiones frente al original

El original delegaba geometría, materiales, cámara y renderizado a Three.js. La versión de curso expone deliberadamente esos pasos. Se conservaron la navegación, las localizaciones y las reglas generales del viaje; se simplificaron los efectos cinematográficos y los escenarios de superficie para que puedan estudiarse dentro de un renderizador por software.
