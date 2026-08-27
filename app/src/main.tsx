import { render } from "solid-js/web";
import App from "./App";
import { DesktopWindowModeController } from "./components/DesktopWindowModeController";
import { shouldMountDesktopWindowModeController } from "./desktopWindowMode";
import "./styles.css";
import "./fixtureLimits.css";
import "./mappingStage.css";

const Root = () =>
  shouldMountDesktopWindowModeController(window.location.search) ? (
    <DesktopWindowModeController>
      <App />
    </DesktopWindowModeController>
  ) : (
    <App />
  );

render(() => <Root />, document.getElementById("root") as HTMLElement);
