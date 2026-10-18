// bevy-react ships TypeScript sources that import react-reconciler, but its template does not
// install @types/react-reconciler, so `tsc` needs these stubs (only the library internals use them).
declare module "react-reconciler" {
  namespace Reconciler {
    type ReactContext<T> = import("react").Context<T>;
    type HostConfig<A = any, B = any, C = any, D = any, E = any, F = any, G = any, H = any, I = any, J = any, K = any, L = any, M = any, N = any> = any;
  }
  function Reconciler(config: any): any;
  export default Reconciler;
}

declare module "react-reconciler/constants" {
  export const DefaultEventPriority: number;
  export const NoEventPriority: number;
}
