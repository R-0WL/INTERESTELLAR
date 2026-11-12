# Esqueleto del menú de configuración

## Acceso

- Antes de iniciar: opción `CONFIGURACION` en el menú principal.
- Durante la exploración: botón de engranaje debajo del minimapa o tecla `G`.
- `Backspace` vuelve a la pantalla desde la que se abrió la configuración.

## Opciones almacenadas en la sesión

| Opción | Valores | Estado inicial |
|---|---|---|
| Música | Activada / desactivada | Activada |
| Diálogos visibles | Activados / desactivados | Activados |
| Efectos | Activados / desactivados | Activados |
| Renderización | Capas de shader 1 a 6 | Capa 1 |

La capa 1 conserva el renderizado actual, sin capas adicionales. Las capas 2–6 están reservadas para trabajo futuro. Los controles actualmente solo permiten cambiar y mostrar la selección; no modifican la reproducción de audio, los textos de diálogo, los efectos visuales ni la ruta de sombreado.

## Controles

- `Up` / `Down`: seleccionar opción.
- `Left` / `Right`: cambiar el valor.
- `Enter`: alternar una opción o avanzar a la siguiente capa.
- `Backspace`: regresar.
