# Sistema Solar v4

## Planetas incluidos

El sistema incluye Mercurio, Venus, Tierra, Marte, Júpiter, Saturno, Urano y Neptuno. Cada uno tiene una malla esférica, material procedural y radio visual adaptado al estilo de planetas gigantes. En la actualización visual, solo Saturno conserva sus anillos; Urano queda como esfera sin aros.

## Escala de distancias

`AU_SCALE = 8,000` unidades de mundo por UA. Las distancias medias al Sol son 0.387, 0.723, 1.000, 1.524, 5.203, 9.537, 19.191 y 30.069 UA, respectivamente. Las posiciones se inicializan sobre una línea radial para orientar el recorrido, pero cada planeta se mueve después sobre una órbita circular completa.

La escala conserva las proporciones orbitales, no una escala kilométrica conjunta: los planetas siguen teniendo radios gigantes a propósito.

## Tiempo orbital

La velocidad angular es `omega = 2*pi/P`. La Tierra utiliza `P = 300 s`. Para los demás planetas se calcula `P = 300 * a^(3/2)`, que mantiene la relación de Kepler entre semieje mayor y periodo. La Luna conserva el cociente entre su periodo sideral y el año sideral terrestre.

## Cinturón de asteroides

- Límite interior: `2.1 UA`.
- Límite exterior: `3.3 UA`.
- Cantidad: `900` rocas.
- Distribución: cobertura angular completa con ángulo áureo y radio distribuido uniformemente por área.
- Movimiento: diferencial; cada roca usa un periodo de Kepler según su distancia. En los bordes de 2.1 y 3.3 UA, los periodos aproximados son 15.2 y 30 minutos.
- Visualización: las rocas individuales componen el cinturón sin curvas guía circulares.

Los asteroides se dibujan en coordenadas locales respecto al Sol; el punto `ASTEROID BELT` de `Game` solo es un marcador de navegación para HUD y minimapa.

## Límites de la simulación

Las órbitas son circulares y coplanares para mantener la implementación adecuada a los contenidos del curso. No se resuelven fuerzas gravitatorias entre planetas, excentricidad, inclinación orbital, resonancias ni efemérides. Los periodos son proporcionales a la ley de Kepler, no fechas astronómicas ni simulación de posiciones reales en una época específica.

## Referencia astronómica

Las distancias medias al Sol, los periodos orbitales y los diámetros relativos se consultaron en la ficha de datos planetarios de NASA Goddard: https://nssdc.gsfc.nasa.gov/planetary/factsheet/ (actualización indicada en la página: 18 de marzo de 2025). Los valores del juego están redondeados y comprimidos para navegación.
