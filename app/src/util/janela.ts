/** Ajustes mínimos da janela Tauri. No navegador/testes, falham silenciosamente. */
async function janela() {
  const [{ getCurrentWindow, currentMonitor }, { LogicalSize }] = await Promise.all([
    import("@tauri-apps/api/window"),
    import("@tauri-apps/api/dpi"),
  ]);
  return { app: getCurrentWindow(), currentMonitor, LogicalSize };
}

export async function prepararJanelaEntrada() {
  try {
    const { app, LogicalSize } = await janela();
    await app.unmaximize();
    await app.setResizable(false);
    await app.setMaximizable(false);
    await app.setSize(new LogicalSize(1020, 680));
    await app.center();
  } catch {
    // Vite/jsdom não têm uma janela Tauri. A experiência web continua funcional.
  }
}

export async function prepararJanelaWorkspace() {
  try {
    const { app, currentMonitor, LogicalSize } = await janela();
    await app.setResizable(true);
    await app.setMaximizable(true);
    await app.setMinSize(new LogicalSize(960, 600));
    const monitor = await currentMonitor();
    const escala = monitor?.scaleFactor ?? 1;
    const larguraDisponivel = monitor ? monitor.size.width / escala : 1600;
    const alturaDisponivel = monitor ? monitor.size.height / escala : 1000;
    await app.setSize(new LogicalSize(Math.min(1600, larguraDisponivel * 0.92), Math.min(960, alturaDisponivel * 0.9)));
    await app.center();
  } catch {
    // Sem efeito fora do Tauri.
  }
}
