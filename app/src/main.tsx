import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createHashRouter, RouterProvider } from "react-router-dom";
import "./estilos.css";
import { Layout } from "./rotas/Layout";
import { PaginaCasos } from "./rotas/PaginaCasos";
import { PaginaNovoCaso } from "./rotas/PaginaNovoCaso";
import { PaginaCaso } from "./rotas/PaginaCaso";
import { PaginaExtracao } from "./rotas/PaginaExtracao";
import { PaginaConversas } from "./rotas/PaginaConversas";
import { PaginaBusca } from "./rotas/PaginaBusca";

const clienteConsultas = new QueryClient({
  defaultOptions: {
    // Dados locais: sem refetch automático por foco/rede; retry só atrapalha erros de validação.
    queries: { refetchOnWindowFocus: false, retry: false },
    mutations: { retry: false },
  },
});

// Hash router: funciona com o protocolo interno do Tauri sem configuração de servidor.
const roteador = createHashRouter([
  {
    element: <Layout />,
    children: [
      { index: true, element: <PaginaCasos /> },
      { path: "casos/novo", element: <PaginaNovoCaso /> },
      { path: "casos/:casoId", element: <PaginaCaso /> },
      { path: "casos/:casoId/extracoes/:extracaoId", element: <PaginaExtracao /> },
      { path: "casos/:casoId/conversas", element: <PaginaConversas /> },
      { path: "casos/:casoId/conversas/:conversaId", element: <PaginaConversas /> },
      { path: "casos/:casoId/busca", element: <PaginaBusca /> },
    ],
  },
]);

createRoot(document.getElementById("raiz")!).render(
  <StrictMode>
    <QueryClientProvider client={clienteConsultas}>
      <RouterProvider router={roteador} />
    </QueryClientProvider>
  </StrictMode>,
);
