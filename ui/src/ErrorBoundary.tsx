import { Component, type ErrorInfo, type ReactNode } from "react";

import { bevy } from "./bevy";

type Props = { children: ReactNode; fallback: (error: Error) => ReactNode };
type State = { error: Error | null };

// React has no hook for error boundaries, so this one component is a class.
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    bevy.ui.error({ message: String(error?.message ?? error), stack: info.componentStack ?? undefined });
  }

  render() {
    return this.state.error ? this.props.fallback(this.state.error) : this.props.children;
  }
}
