# Interstellar Journey — Rust Software Renderer

Adaptación en Rust del prototipo web **Interstellar Journey**. La experiencia conserva el recorrido espacial y sus mecánicas principales, pero sustituye React, Three.js y React Three Fiber por un renderizador por software construido en el propio proyecto.

La implementación sigue la secuencia didáctica visible en el repositorio del curso **cc2018 — Gráficas por Computadora** de la UVG: `Framebuffer`, primitivas, relleno de triángulos, ventana y render loop; después, geometría 3D, sombreado, coordenadas baricéntricas, transformaciones, cámara y funciones de fragmento.

## Qué incluye

- Vuelo libre en tercera persona con aceleración, frenado, giro, cabeceo y turbo.
- Recorrido por la Tierra, Luna, Marte, cinturón de asteroides, Júpiter y Saturno.
- Agujero de gusano que habilita el sistema de Gargantúa.
- Gargantúa, acoplamiento, Miller, Mann y Edmund.
- Aterrizaje y despegue en mundos habilitados.
- Atracción gravitatoria y destrucción al entrar demasiado en Gargantúa.
- Planetas con patrones procedurales, anillos, estrellas y efectos del agujero de gusano.
- HUD dibujado en el framebuffer.
- Guardado de capturas BMP con `P`.
- Importador OBJ de posiciones, coordenadas UV, normales y caras poligonales trianguladas.

## Requisitos

- Rust y Cargo (edición 2021).
- Un entorno de escritorio compatible con `minifb`.

## Ejecutar

Desde la raíz del proyecto:

```bash
cargo run --release
```

Para probar otro modelo OBJ como geometría base de los planetas:

```bash
cargo run --release -- --obj ruta/al/modelo.obj
```

El proyecto incluye `assets/models/esfera.obj`, convertido desde el archivo `esfera.glb` suministrado. También conserva el GLB original en `assets/models/source_esfera.glb`.

## Controles

| Tecla | Acción |
|---|---|
| `W` | Acelerar |
| `S` | Frenar |
| `A` / `D` o `←` / `→` | Girar |
| `↑` / `↓` | Cabecear |
| `Shift` | Turbo |
| `E` | Aterrizar cerca de un mundo habilitado |
| `Space` | Cruzar el agujero de gusano cuando esté cerca |
| `R` | Despegar o reiniciar después de una colisión |
| `P` | Guardar captura BMP en `captures/` |
| `Esc` | Salir |

## Organización del código

```text
src/
├── main.rs         # Ventana, lectura de teclado y render loop
├── math.rs         # Vec3 (nalgebra-glm) y matrices 4x4 explícitas
├── framebuffer.rs # Color buffer, Z-buffer, líneas y escritura BMP
├── mesh.rs         # Mallas procedurales e importador OBJ
├── game.rs         # Datos del viaje, controles y simulación de vuelo
├── renderer.rs     # Proyección, rasterización, Z-buffer y materiales
└── ui.rs           # HUD dibujado en píxeles

assets/models/
├── esfera.obj      # Modelo convertido y usado como malla esférica
├── esfera.mtl      # Material de origen de referencia
└── source_esfera.glb
```

## Relación con las etapas del curso

| Etapa del repositorio | Aplicación en este proyecto |
|---|---|
| `00-HELLO-WORLD` | Cargo, módulos y punto de entrada `main.rs`. |
| `01-SR-01-POINT` | `Framebuffer::set_pixel` y buffer de color. |
| `02-SR-02-LINE` | Línea de Bresenham para depuración y HUD. |
| `03-SR-03-POLYGON` | Mallas representadas por listas de triángulos. |
| `04-SR-04-FILLED-POLYGON` | Relleno por coordenadas baricéntricas y vectores `Vec3`. |
| `05-SR-05-WINDOWS` | Ventana mediante `minifb`. |
| `06-SR-06-RENDER-LOOP` | Bucle de actualización, simulación y presentación. |
| `18-RP-01-3D-MODELS` | Importador OBJ y arreglos de vértices. |
| `19-RP-02-FLAT-SHADING` | Normales y luz difusa/especular. |
| `20-RP-03-VALUE-INTERPOLATION` | Interpolación baricéntrica de profundidad, UV, posición y normal. |
| `21-RP-04-MATRIX-TRANSFORMATIONS` | Matrices de modelo, vista y proyección en `math.rs`. |
| `22-RP-05-ORBIT-CAMERA` | Cámara de persecución y cámara de superficie. |
| `23-RP-06-FRAGMENT-SHADER` | `sample_material` y `fragment`, ejecutados en CPU. |
| `24-RP-07-ANIMATED-FRAGMENT-SHADER` | Patrones dependientes del tiempo para anillos, agua y efectos. |
| `25-RP-08-COMBINED-FRAGMENT-SHADER` | Materiales distintos para varios cuerpos de la escena. |
| `26-RP-09-NOISE-FRAGMENT-SHADER` | Ruido determinista para continentes, terreno y variaciones. |

> **Nota de alcance:** esta versión prioriza la ruta Software Rendering → Render Pipeline, que es la más directa para representar el mundo 3D del proyecto. No intenta replicar el stack completo de React/Three.js, ni implementar un motor genérico, físicas orbitales reales, mapas de textura externos o clipping geométrico avanzado. El sombreado es procedural y aproximado.

## Estructura de aprendizaje recomendada

1. Ejecuta la aplicación y modifica el tamaño del framebuffer.
2. Cambia `Framebuffer::draw_line` o `Mesh::add_flat_triangle` para observar las primitivas.
3. Inspecciona `Renderer::draw_mesh` y `rasterize_triangle` para seguir el pipeline.
4. Cambia `Mat4::look_at` y `Mat4::perspective` para experimentar con la cámara.
5. Modifica `sample_material` para crear otros planetas y efectos.
6. Modifica `Game::update` para ajustar la física y las reglas de interacción.

## Compilación y pruebas

```bash
cargo fmt
cargo test
cargo run --release
```

El entorno donde se preparó esta adaptación no cuenta con `rustc` ni `cargo`, así que no fue posible ejecutar la compilación aquí. Los comandos anteriores son los pasos de verificación que deben correrse en una instalación local de Rust.
