import {
  AppWindow,
  BadgeCheck,
  Boxes,
  Building2,
  Cloud,
  FileCog,
  FileText,
  Fingerprint,
  Gauge,
  KeyRound,
  Layers,
  Lock,
  MonitorSmartphone,
  Rocket,
  ScrollText,
  ShieldCheck,
  SlidersHorizontal,
  Smartphone,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";

/** Map a catalog group name to a representative Lucide icon. */
export const groupIcons: Record<string, LucideIcon> = {
  "Device Configuration": SlidersHorizontal,
  Compliance: BadgeCheck,
  "Endpoint Security": ShieldCheck,
  Apps: AppWindow,
  Scripts: TerminalSquare,
  "Windows Enrollment": MonitorSmartphone,
  "Windows Updates": Rocket,
  "Apple Enrollment": Smartphone,
  "Endpoint Analytics": Gauge,
  "Policy Sets": Boxes,
  "Conditional Access": Lock,
  "Tenant Administration": Building2,
  "Azure AD": Cloud,
};

export function groupIcon(group: string): LucideIcon {
  return groupIcons[group] ?? Layers;
}

/** Map a specific object type id to an icon (falls back to its group icon). */
export const typeIcons: Record<string, LucideIcon> = {
  ConditionalAccess: Lock,
  NamedLocations: Fingerprint,
  RoleDefinitions: KeyRound,
  Notifications: FileText,
  SettingsCatalog: FileCog,
  ComplianceScripts: ScrollText,
};
