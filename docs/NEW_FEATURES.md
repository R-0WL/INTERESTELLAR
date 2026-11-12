# Cambios de interfaz, escala y movimiento orbital (v4)

## Escala gigante

Los radios de los cuerpos principales se multiplicaron por `PLANET_SCALE = 4.0` respecto de v2. La nave mantiene su tamaño, por lo que es claramente pequeña frente a los mundos y puede maniobrar alrededor de ellos.

| Cuerpo | Radio v2 | Radio v3 |
|---|---:|---:|
| Tierra | 180 | 720 |
| Luna | 54 | 216 |
| Marte | 105 | 420 |
| Júpiter | 350 | 1,400 |
| Saturno | 300 | 1,200 |
| Sol | 300 | 1,200 |
| Gargantúa | 185 | 740 |
| Miller | 135 | 540 |
| Mann | 115 | 460 |
| Edmund | 125 | 500 |

Las distancias entre centros también se ampliaron deliberadamente. El recorrido no sigue unidades astronómicas reales: usa un corredor más largo y legible para evitar que los planetas gigantes se amontonen.

## Movimiento orbital

Las posiciones se actualizan cada frame de simulación, no solo se dibujan las órbitas.

- Tierra, Marte, Júpiter y Saturno orbitan el Sol.
- La Luna orbita la Tierra.
- Miller, Mann y Edmund orbitan Gargantúa.
- Los planetas exteriores tienen velocidades angulares menores; la velocidad tangencial resultante mantiene un desplazamiento lento pero perceptible.
- El minimapa, la detección de proximidad y el HUD consultan las posiciones orbitales actualizadas.

Los movimientos son circulares y simplificados, adecuados para el proyecto didáctico; no representan una simulación gravitatoria astronómica.

## Minimapa

- `M` muestra u oculta el minimapa de la esquina superior derecha.
- La posición de la nave y los cuerpos se actualiza junto con la simulación.
- Las guías circulares reflejan los radios orbitales actuales del sistema.
- El mapa cambia de centro (Sol / Gargantúa) después del agujero de gusano.

## Menú

- `↑` / `↓` cambian el modo seleccionado.
- `Enter` abre el modo seleccionado.
- Exploración inicia el recorrido libre. Historia y Niveles siguen siendo pantallas de presentación hasta desarrollar sus campañas.
- `Backspace` vuelve al menú principal durante la exploración.


## Sistema Solar completo (v4)

Se añadieron Mercurio, Venus, Urano y Neptuno para completar los ocho planetas. Cada uno cuenta con un shader procedural; Urano se muestra como esfera sin anillos visibles. Mercurio y Venus usan materiales procedurales propios.

## Distancias y periodos

- `AU_SCALE = 8,000`: las distancias orbitales usan proporciones cercanas a los semiejes mayores en UA.
- La Tierra completa exactamente una vuelta cada 300 segundos simulados.
- Los periodos restantes se calculan como `P = 300 * a^(3/2)` para mantener la relación de Kepler.
- La Luna usa el cociente real entre el periodo lunar sideral y el año sideral terrestre.
- El agujero de gusano se movió más allá de Neptuno; el sistema de Gargantúa se desplazó para conservar la secuencia de transición.

## Cinturón de asteroides completo

El cinturón cubre 360 grados entre 2.1 y 3.3 UA, con 900 rocas distribuidas por toda el área del anillo. Cada roca se mueve alrededor del Sol con su propia velocidad de Kepler: las interiores avanzan más rápido que las exteriores. Se dibujan usando coordenadas relativas al Sol. La actualización visual sustituye el octaedro procedural por el objeto `Asteroid.glb` y elimina las guías circulares del cinturón.

## Pruebas añadidas

- Existencia de los ocho planetas.
- Periodo terrestre de 300 segundos y regreso al punto de inicio después de una vuelta.
- Orden creciente de los periodos orbitales exteriores.
- Proporciones de distancias en UA.
- Cobertura angular de los 36 sectores del cinturón y límites de su radio.


## Cambios posteriores (v5)

La ubicación del agujero de gusano más allá de Neptuno descrita arriba corresponde al estado de v4 y fue reemplazada: en v5 queda anclado junto a Saturno. El superboost Shift+B y el efecto warp se documentan en `WORMHOLE_AND_SUPERBOOST_V5.md`.
