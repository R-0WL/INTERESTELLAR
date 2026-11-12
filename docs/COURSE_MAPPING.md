# Mapa de conceptos del curso

Este documento conecta las etapas identificadas en `cc2018-2026-02-10` con los módulos de esta adaptación.

## Software Rendering

- **Framebuffer:** `framebuffer.rs` administra el arreglo de píxeles y el Z-buffer.
- **Puntos y líneas:** `set_pixel` y `draw_line`.
- **Polígonos:** las mallas se almacenan como arreglos de triángulos indexados.
- **Relleno:** `rasterize_triangle` recorre el bounding box y calcula pesos baricéntricos.
- **Ventana y render loop:** `main.rs` usa `minifb` para entrada y presentación.
- **UI por píxeles:** `ui.rs` construye el menú principal, las pantallas de modo, el HUD y el minimapa directamente en el framebuffer.
- **Minimapa:** las posiciones 3D de los cuerpos se transforman a coordenadas 2D; las órbitas circulares se dibujan con segmentos de línea.

## Render Pipeline

- **Modelo:** cada objeto aplica traslación, rotación y escala.
- **Vista:** `Mat4::look_at` transforma el mundo al espacio de cámara.
- **Proyección:** `Mat4::perspective` genera coordenadas homogéneas.
- **Viewport:** el rasterizador transforma NDC a coordenadas de pantalla.
- **Modelos:** `load_obj` lee posiciones, UV, normales y caras.
- **Interpolación:** `rasterize_triangle` interpola profundidad y atributos con corrección de perspectiva.
- **Iluminación:** `fragment` combina luz ambiental, difusa y especular.
- **Fragment shader:** `sample_material` genera patrones procedurales por tipo de cuerpo.
- **Órbitas:** `Game::build_orbits` deriva las velocidades angulares con la tercera ley de Kepler y `Renderer::render_solar_orbits` proyecta los segmentos circulares de los ocho planetas, los límites del cinturón y la órbita lunar.
- **Distribución procedural:** `make_asteroids` usa un ángulo áureo y una distribución radial por área para cubrir 360 grados con 900 rocas. Cada instancia usa el modelo importado de `Asteroid.glb` con variaciones de escala.

## Decisiones frente al original

El original delegaba geometría, materiales, cámara y renderizado a Three.js. La versión de curso expone deliberadamente esos pasos. Se conservaron la navegación, las localizaciones y las reglas generales del viaje. La simulación se concentra en el vuelo orbital, la gravedad estilizada, la evitación predictiva y las vistas globales dentro del renderizador por software.
