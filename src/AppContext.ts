import { createContext } from "react";
import { AppContextValue } from "./types";

const AppContext = createContext<AppContextValue>({
  addIgnoreDragTarget: () => {
    throw new Error("AppContext is not implemented");
  },
  removeIgnoreDragTarget: () => {
    throw new Error("AppContext is not implemented");
  }
})

export default AppContext;
