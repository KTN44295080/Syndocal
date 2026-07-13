import { render } from "solid-js/web";
import App from "./App";
import { DesktopWindowModeController } from "./components/DesktopWindowModeController";
import "./styles.css";

render(
  () => (
    <DesktopWindowModeController>
      <App />
    </DesktopWindowModeController>
  ),
  document.getElementById("root") as HTMLElement,
);
