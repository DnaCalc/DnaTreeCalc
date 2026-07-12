import { createBrowserRouter } from "react-router";
import { WarmEditorialWorkbench } from "./directions/WarmEditorialWorkbench";
import { AnalyticalCompareStudio } from "./directions/AnalyticalCompareStudio";
import { ModularEvidenceCockpit } from "./directions/ModularEvidenceCockpit";
import { RefinedWarmEditorial } from "./directions/RefinedWarmEditorial";
import { DenseInformationView } from "./directions/DenseInformationView";
import { InformationArchitecture } from "./directions/InformationArchitecture";
import { FocusedExploreMode } from "./directions/FocusedExploreMode";
import { FocusedInspectMode } from "./directions/FocusedInspectMode";
import { FocusedWorkbenchMode } from "./directions/FocusedWorkbenchMode";
import { DirectionSelector } from "./components/DirectionSelector";

export const router = createBrowserRouter([
  {
    path: "/",
    Component: DirectionSelector,
  },
  {
    path: "/explore",
    Component: FocusedExploreMode,
  },
  {
    path: "/inspect",
    Component: FocusedInspectMode,
  },
  {
    path: "/workbench",
    Component: FocusedWorkbenchMode,
  },
  {
    path: "/architecture",
    Component: InformationArchitecture,
  },
  {
    path: "/refined",
    Component: RefinedWarmEditorial,
  },
  {
    path: "/dense",
    Component: DenseInformationView,
  },
  {
    path: "/warm-editorial",
    Component: WarmEditorialWorkbench,
  },
  {
    path: "/analytical-compare",
    Component: AnalyticalCompareStudio,
  },
  {
    path: "/modular-evidence",
    Component: ModularEvidenceCockpit,
  },
]);