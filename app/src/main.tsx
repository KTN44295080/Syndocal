import { render } from "solid-js/web";
import App from "./App";
import { DesktopWindowModeController } from "./components/DesktopWindowModeController";
import { shouldMountDesktopWindowModeController } from "./desktopWindowMode";
import "./styles.css";
import "./fixtureLimits.css";
import "./mappingStage.css";
import "./controls.css";
import { installRangeFillSync } from "./rangeFill";

const Root = () =>
  shouldMountDesktopWindowModeController(window.location.search) ? (
    <DesktopWindowModeController>
      <App />
    </DesktopWindowModeController>
  ) : (
    <App />
  );

installRangeFillSync();

render(() => <Root />, document.getElementById("root") as HTMLElement);
