# Escala y Gargantúa — v6

## Escala de los cuerpos

`PLANET_SCALE` pasa de `4.0` a `15.0` respecto de los radios base. Se aplica a los ocho planetas, el Sol y el agujero de gusano. Gargantúa usa una escala propia derivada del radio externo de su disco y la separación deseada respecto de Miller. La nave no cambia de tamaño.

La escala orbital pasa de `1 UA = 8,000` a `1 UA = 30,000` unidades. Esto mantiene las proporciones aproximadas entre semiejes orbitales y conserva holguras grandes entre planetas. El spawn inicial se desplazó al corredor entre Tierra y Venus para evitar aparecer dentro de alguno de los dos cuerpos ampliados.

La órbita de la Luna se amplió a 6,500 unidades para que no intersecte la Tierra ampliada.

## Agujero de gusano junto a Saturno

El agujero de gusano mantiene su posición relativa a Saturno, ahora con desplazamiento `(18,000, 3,900, 0)`. Este punto queda fuera del límite exterior de los anillos de Saturno, incluso con la nueva escala.

## Gargantúa

Gargantúa usa `assets/blackhole_dorado/disco_capas.obj`, el disco de gas de 22 capas exportado desde Blender. Su ancho es 5.2 unidades locales (radio exterior 2.6), por lo que se escala hasta `GARGANTUA_DISK_RADIUS = 83,400` unidades de juego. El borde del disco queda a 1,600 unidades de la órbita interior de Miller (85,000). El horizonte se calcula en postprocesado con `GARGANTUA_RADIUS = 35,445` (0.425 del radio exterior), y no depende de una esfera dentro del OBJ de gas.

La simulación trabaja con unidades de mundo estilizadas, no con kilómetros físicos; por tanto, el valor 1,600 representa la separación solicitada dentro de la escala del juego y no una conversión astronómica literal.

Las órbitas de los planetas del segundo sistema quedan muy alejadas del centro:

| Planeta | Radio orbital |
|---|---:|
| Miller | 85,000 |
| Mann | 155,000 |
| Edmund | 235,000 |

La salida del agujero de gusano está a 275,000 unidades del centro, fuera de la órbita de Edmund. La entrada al sistema permite acercarse desde el planeta exterior hacia Gargantúa. Los periodos de esos tres cuerpos mantienen una relación tipo Kepler entre sus radios orbitales.

El archivo `blackhole.obj` se genera a partir de `blackhole.glb` mediante `tools/convert_glb_assets.py`; el ejecutable usa el OBJ ya preparado y no necesita dependencias Python para cargarlo.

## Disco y lente gravitacional

Las 22 capas se rasterizan como superficies aditivas: usan el alpha de los PNG, interpolación bilineal de UV y conversión sRGB/lineal; se comprueba la profundidad de geometría opaca, pero las capas no escriben profundidad. El eje Z local del OBJ se convierte al eje Y del mundo mediante una rotación de -90 grados en X.

Después de dibujar cuerpos, nave y efectos de velocidad, el postprocesado proyecta el centro de Gargantúa y el borde del horizonte a coordenadas de pantalla. Solo itera dentro de un radio de influencia de 3.5 veces el radio aparente del horizonte, desplaza radialmente las coordenadas de muestreo con fuerza 2.15, usa interpolación bilineal y pinta el horizonte de negro. Antes de deformar cada píxel compara el Z-buffer con la profundidad del agujero negro, así que no altera geometría más cercana.

El búfer fuente reutilizable evita crear una nueva asignación por fotograma. El coste del lente depende del área proyectada, no del número de triángulos del resto del sistema solar. Esta implementación es un lente radial aproximado en espacio de pantalla, no una integración relativista completa.

## Renderizado y visibilidad

El plano lejano de la proyección se amplió a 3,000,000 unidades y las estrellas de fondo se sitúan a 1,300,000. Las vistas generales elevan el culling de cuerpos y asteroides a 2,900,000 unidades para que Neptuno y el sistema exterior permanezcan visibles en los encuadres completos.

## Pruebas añadidas/actualizadas

- Escala visual 15x para Tierra, Sol y agujero de gusano; Gargantúa usa una escala derivada de la separación disco-órbita.
- Separación nave-planetas en el punto de inicio.
- Separación de los cuerpos de Gargantúa respecto del volumen central y margen de 1,600 unidades entre disco y órbita de Miller.
- Salida del agujero de gusano fuera de la órbita exterior.
- Offset fijo del agujero de gusano respecto a Saturno.
