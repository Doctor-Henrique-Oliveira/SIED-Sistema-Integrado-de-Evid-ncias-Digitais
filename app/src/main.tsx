import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createHashRouter, RouterProvider } from "react-router-dom";
import "./estilos.css";
import { Layout } from "./rotas/Layout";
import { PaginaCasos } from "./rotas/PaginaCasos";
import { PaginaNovoCaso } from "./rotas/PaginaNovoCaso";
import { PaginaCaso } from "./rotas/PaginaCaso";
import { PaginaDispositivos } from "./rotas/PaginaDispositivos";
import { PaginaExtracoes } from "./rotas/PaginaExtracoes";
import { PaginaExtracao } from "./rotas/PaginaExtracao";
import { PaginaConversas } from "./rotas/PaginaConversas";
import { PaginaBusca } from "./rotas/PaginaBusca";
import { PaginaArquivos, PaginaContatos, PaginaMidias } from "./rotas/PaginasModulos";
import {
  PaginaConfiguracoes,
  PaginaConsultas,
  PaginaLocalizacoes,
  PaginaRelatorios,
} from "./rotas/PaginasEstruturais";

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
      { path: "configuracoes", element: <PaginaConfiguracoes /> },
      { path: "casos/novo", element: <PaginaNovoCaso /> },
      { path: "casos/:casoId", element: <PaginaCaso /> },
      { path: "casos/:casoId/dispositivos", element: <PaginaDispositivos /> },
      { path: "casos/:casoId/extracoes", element: <PaginaExtracoes /> },
      { path: "casos/:casoId/extracoes/:extracaoId", element: <PaginaExtracao /> },
      { path: "casos/:casoId/conversas", element: <PaginaConversas /> },
      { path: "casos/:casoId/conversas/:conversaId", element: <PaginaConversas /> },
      { path: "casos/:casoId/busca", element: <PaginaBusca /> },
      { path: "casos/:casoId/midias", element: <PaginaMidias /> },
      { path: "casos/:casoId/arquivos", element: <PaginaArquivos /> },
      { path: "casos/:casoId/contatos", element: <PaginaContatos /> },
      { path: "casos/:casoId/localizacoes", element: <PaginaLocalizacoes /> },
      { path: "casos/:casoId/consultas", element: <PaginaConsultas /> },
      { path: "casos/:casoId/relatorios", element: <PaginaRelatorios /> },
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
