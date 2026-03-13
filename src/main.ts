import { mount } from "svelte";
import App from "./App.svelte";

const appTarget = document.getElementById("app");
if (!appTarget) throw new Error("Cannot find #app element");
const app = mount(App, { target: appTarget });
export default app;
