# Hoja de ruta de NaAnSentinels

**Idiomas:** [English](../en/roadmap.md) · [Français](../fr/feuille-de-route.md) · Español

## Hecho

- Motor de análisis estático: estructura PE, secciones, funciones importadas, entropía, SHA-256, desensamblado x86/x64.
- Veredicto explicado, ninguna acción destructiva.
- Antiphishing sin conexión (correo pegado o archivo `.eml`).
- Reglas y configuración editables, recargables desde la aplicación.
- Vista simple animada y vista experta con esquemas.
- Señal de presencia opcional y anónima, cuenta opcional.
- Servidor: cuentas, roles (usuario `utilisateur`, analista `analyste`, admin `admin`), panel de control,
  registro de seguridad.
- Firma de las versiones oficiales (Ed25519).

## A continuación

### Antiphishing conectado al buzón de correo

- Conexión IMAP opcional, contraseña de aplicación guardada fuera del repositorio y nunca mostrada.
- Lectura de las cabeceras de autenticación: SPF, DKIM, DMARC.
- Detección de dominios de apariencia similar.

### Ingeniería inversa en profundidad

- Seguimiento de las llamadas desde el punto de entrada y árbol de llamadas.
- Detección de packers conocidos, importaciones diferidas, cadenas cifradas.
- Reglas YARA además del formato propio.
- Verificación de la firma Authenticode de los archivos analizados.
- Consulta de una base de reputación por huella, sin enviar el archivo.

### Vigilancia

- Vigilancia de una carpeta (Descargas): análisis de cada programa nuevo antes de abrirlo.
- Cuarentena local reversible, siempre por decisión del usuario.
- Análisis de una carpeta entera con resumen.

### Servidor

- Límites de tasa persistentes y compatibilidad con un proxy de confianza.
- Autenticación de dos factores, restablecimiento de contraseña.
- Historial de los dispositivos conectados a lo largo del tiempo.

## Fuera del alcance (asumido)

- Sin controlador de kernel, sin interceptación de la ejecución a nivel del sistema, sin acciones de administrador.
  NaAnSentinels sigue siendo una herramienta en espacio de usuario que **analiza y explica**.
- No sustituye al antivirus del sistema: lo complementa.
