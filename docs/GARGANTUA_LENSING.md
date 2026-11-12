# Gargantúa: disco de acreción y lente gravitacional

## Recursos integrados

El proyecto carga `assets/blackhole_dorado/disco_capas.obj`, un OBJ con 22 planos UV. Los PNG `disco_centro.png`, `disco_medio.png` y `disco_borde.png` conservan las texturas RGBA de Blender. Sus versiones `.rgba` contienen los mismos píxeles sin compresión y se incluyen con `include_bytes!`, de modo que el renderizador no requiere una biblioteca PNG en tiempo de ejecución.

El OBJ está en Z-up. Se convierte al sistema Y-up del juego con una rotación de -90 grados sobre X. Su ancho local de 5.2 se escala usando `GARGANTUA_DISK_RADIUS / 2.6`, conservando el borde a 83,400 unidades y la separación de 1,600 respecto a la órbita de Miller.

Las capas usan las opacidades del `capas.json`: 0.036 para borde, 0.057 para medio y 0.078 para centro. Se toma el alpha como máscara, se interpola la textura bilinealmente, se convierte RGB de sRGB a luz lineal y se suma de forma aditiva con intensidad 5. El pase prueba la profundidad frente a geometría opaca, pero no escribe en el Z-buffer para permitir que todas las capas se acumulen.

## Postprocesado gravitacional

Al final del render 3D, una vez dibujadas la nave y las estelas, el renderizador proyecta el centro de Gargantúa usando las matrices `Mat4` del propio motor. Proyecta también un punto separado por `GARGANTUA_RADIUS` a lo largo del eje derecho de cámara para obtener el radio aparente del horizonte en píxeles.

Solo se recorre la caja delimitadora de radio `r_max = 3.5 * r_s`. Para cada píxel:

- Si queda dentro del horizonte aparente, se pinta negro absoluto.
- Si está en la zona de lente, se remuestrea el framebuffer fuente en `p - dirección * (r_s² / d) * 2.15 * falloff`.
- La lectura usa interpolación bilineal y se añade un halo blanco cálido alrededor del borde.
- Si el Z-buffer indica que la geometría del píxel está delante de Gargantúa, el píxel no se modifica.

Se reutiliza `Framebuffer::postprocess_source` para no asignar un nuevo vector cada fotograma. El efecto trabaja en espacio de pantalla y su coste depende de los píxeles dentro de la caja proyectada, no de la geometría del resto del sistema solar.

## Alcance y validación

La deformación es una aproximación radial 2D; no es un trazado de rayos relativista completo. El proyecto original solo declara `minifb`, `nalgebra-glm` y `font8x8` en `Cargo.toml`; no declara `wide` ni `safe_arch`, así que esta integración usa el camino escalar sin añadir dependencias SIMD.

Se añadió una prueba unitaria que verifica que el horizonte queda negro, que un píxel con profundidad de primer plano se conserva y que el exterior de la caja no cambia.
