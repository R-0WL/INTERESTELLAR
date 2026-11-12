# Cambios v5: agujero de gusano y superboost

## Agujero de gusano junto a Saturno

El agujero de gusano dejó de ser un objeto estático ubicado después de Neptuno. Ahora mantiene un desplazamiento local fijo respecto a Saturno (`wormhole_saturn_offset`), por lo que lo acompaña mientras Saturno recorre su órbita. El minimapa usa su posición actual del mundo, igual que con los planetas.

## Boost

- `W + Shift`: boost normal con límite de 1000 unidades/s.
- `W + Shift + B`: superboost con límite de 2500 unidades/s.
- `B` sin `Shift` no activa el superboost.

La aceleración se interpola para no producir un salto brusco de velocidad. El estado de los boosts se guarda en `Game` para que el renderizador pueda dibujar sus efectos sin mezclar entrada, física y presentación.

La actualización de modelos usa Endurance + Ranger + Ranger durante el Sistema Solar y Ranger después del cruce. Gargantúa y el punto de acoplamiento usan sus modelos GLB convertidos a OBJ. El agujero de gusano es una sola esfera; ya no se dibujan los aros de la Tierra, el Sol ni Urano. Saturno conserva sus anillos.

## Efecto visual warp

El framebuffer incorpora dibujo de líneas con alpha blending. Con el boost normal se dibujan pocas líneas radiales de opacidad muy baja. Al combinar el boost y el superboost aparece un túnel radial de líneas blancas y azules, inspirado en la imagen de referencia, dejando libre el centro para no ocultar la nave. El HUD se dibuja después del renderizador y permanece legible.

## Archivos modificados

- `src/game.rs`: anclaje del agujero de gusano, entrada del superboost, límite de velocidad y pruebas.
- `src/main.rs`: lectura de la tecla `B`.
- `src/framebuffer.rs`: alpha blending y líneas semitransparentes.
- `src/renderer.rs`: efecto radial con distinta intensidad para cada boost.
- `src/ui.rs`: telemetría y controles actualizados.
