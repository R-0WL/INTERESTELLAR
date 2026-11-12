# Actualización visual de modelos y velocidad

## Modelos sustituidos

- **Trayecto del Sistema Solar:** la nave usa `endurance + ranger + ranger.glb`.
- **Después del agujero de gusano:** la nave cambia a `ranger_ship_of_interstellar_free.glb`.
- **Gargantúa:** se usa `blackhole.glb` en lugar de la esfera procedural.
- **Acoplamiento de Endurance:** el aro procedural se sustituye por `endurance + ranger.glb`.
- **Cinturón de asteroides:** 900 instancias del modelo `Asteroid.glb`, con tamaños y proporciones independientes. Hay fragmentos pequeños, medianos y algunos grandes; se eliminaron las dos guías circulares que delimitaban el cinturón.

Los GLB se convierten a OBJ compactos con `tools/convert_glb_assets.py`, porque el cargador del renderizador por software consume OBJ. El juego carga los OBJ ya incluidos y no necesita instalar bibliotecas Python para ejecutarse. Para regenerarlos se requieren `trimesh`, `numpy` y `scipy`.

## Anillos visibles

Se eliminaron los aros de la Tierra, el Sol, Urano y el agujero de gusano. Saturno conserva sus anillos y Gargantúa mantiene la geometría de acreción incluida en su modelo.

## Velocidad

- `W + Shift`: boost normal, máximo **1000 unidades/s**.
- `W + Shift + B`: superboost, máximo **2500 unidades/s**.
- `B` sin `Shift` no activa el superboost.

Los límites están aplicados tanto a la velocidad escalar como al vector de velocidad, y el HUD refleja el límite nuevo.


## Orientación y cámara (referencias visuales)

Los modelos OBJ de vuelo tienen ejes locales distintos. Durante el Sistema Solar, `endurance_ranger_ranger.obj` recibe una rotación local de 90 grados alrededor de Y para presentar el anillo del Endurance de frente, como en la referencia. Después del agujero de gusano, `ranger.obj` recibe una rotación local de 180 grados alrededor de Y para que su motor de popa (ubicado en el extremo local -Z) apunte hacia la cámara. La cámara de persecución sigue el eje frontal de la nave, no la velocidad residual, y mantiene visible la perspectiva trasera durante los giros. Estas correcciones solo rotan el modelo/cámara visual; no cambian las reglas de navegación.

## Gargantúa ampliado

El borde radial del disco importado se escala a 83,400 unidades de mundo frente a la órbita de Miller de 85,000, dejando 1,600 unidades. El radio de colisión se establece como el 42.5% del radio del disco, de acuerdo con la proporción del OBJ normalizado. Las cifras están expresadas en unidades del juego, no kilómetros físicos.

## Configuración (esqueleto)

El menú principal incluye `Configuración`, y durante la exploración se abre con el engranaje bajo el minimapa o con `G`. Los controles de música, visibilidad de diálogos, efectos y capa de shader (1 a 6) guardan selección en memoria durante la sesión. La capa 1 es el renderizador actual; las capas 2 a 6 son marcadores reservados. No están conectados a sistemas de audio, diálogos, efectos ni sombreado todavía.
