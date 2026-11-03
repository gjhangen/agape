// React is loaded from cdnjs as a UMD global; this shim lets source files import it.
export const React: typeof import("react") = (window as any).React;
export const ReactDOM: any = (window as any).ReactDOM;
export const { useState, useEffect, useMemo, useRef, useCallback, useContext, createContext, useSyncExternalStore } = React;
