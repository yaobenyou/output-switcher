import { Card, CardContent, CircularProgress, CssBaseline, Stack } from "@mui/material";
import { window as tauriWindow } from "@tauri-apps/api";
import { LogicalSize, getCurrent } from "@tauri-apps/api/window";
import { useCallback, useEffect, useMemo, useRef } from "react";
import AppContext from "./AppContext";
import Meter from "./Meter";
import ThemeProvider from "./ThemeProvider";
import useWindowsAudioState from "./useWindowsAudioState";

function App() {

  const ignoreDragTargetsRef = useRef<HTMLElement[]>([]);

  const addIgnoreDragTarget = useCallback((target: HTMLElement) => {
    ignoreDragTargetsRef.current.push(target);
  }, []);

  const removeIgnoreDragTarget = useCallback((target: HTMLElement) => {
    const index = ignoreDragTargetsRef.current.indexOf(target);
    if (index !== -1) {
      ignoreDragTargetsRef.current.splice(index, 1);
    }
  }, []);


  const cardRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {

    if (!cardRef.current) {
      return;
    }

    // with padding
    const width = cardRef.current.clientWidth + 32;
    const height = cardRef.current.clientHeight + 32;

    const physicalSize = new LogicalSize(width, height);

    const mainWindow = getCurrent();
    mainWindow.setSize(physicalSize);

    const minSize = new LogicalSize(64, physicalSize.height);
    const maxSize = new LogicalSize(physicalSize.width, physicalSize.height);

    mainWindow.setMinSize(minSize);
    mainWindow.setMaxSize(maxSize);

    const handler = async (e: MouseEvent) => {

      if (ignoreDragTargetsRef.current.some(target => target.contains(e.target as Node))) {
        return;
      }

      await tauriWindow.appWindow.startDragging();
    }

    cardRef.current.addEventListener("mousedown", (handler));

    return () => {
      cardRef.current?.removeEventListener("mousedown", handler);
    }

  }, [])

  const audioState = useWindowsAudioState();

  const defaultDevice = useMemo(() => {
    if (!audioState) {
      return null;
    }

    return audioState.audioDeviceList.find(device => device.id === audioState.default);
  }, [audioState?.default]);

  const getVolume = useCallback((deviceId: string) => {

    if (!audioState) {
      return 0;
    }

    const device = audioState.audioDeviceList.find(device => device.id === deviceId);
    return device?.volume || 0;

  }, [audioState?.audioDeviceList])

  return (
    <ThemeProvider>
      <CssBaseline />
      <AppContext.Provider
        value={{
          addIgnoreDragTarget,
          removeIgnoreDragTarget,
        }}
      >
        <Card ref={cardRef}>
          <CardContent>
            {defaultDevice && (
              <Meter
                device={defaultDevice}
                defaultVolume={getVolume(defaultDevice.id)}
                deviceList={audioState?.audioDeviceList}
              />
            )}

            {!defaultDevice && (
              <Stack spacing={2} alignItems="center">
                <CircularProgress />
              </Stack>
            )}

          </CardContent>
        </Card>
      </AppContext.Provider>
    </ThemeProvider>
  );
}

export default App;
