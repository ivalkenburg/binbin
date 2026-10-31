import { ChevronDown, LogOut, Monitor, Moon, Sun } from "lucide-react";
import type { Theme } from "@/hooks/use-theme";
import type { User } from "@/types/api";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";

const themes = [
  ["system", "System", Monitor],
  ["light", "Light", Sun],
  ["dark", "Dark", Moon],
] as const;

export function AccountMenu({
  user,
  theme,
  onTheme,
  onLogout,
}: {
  user: User;
  theme: Theme;
  onTheme: (theme: Theme) => void;
  onLogout: () => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="outline"
            id="account-menu"
            aria-label={`Account menu for ${user.username}`}
          />
        }
      >
        <span className="avatar" aria-hidden="true">
          {user.username.slice(0, 1).toUpperCase()}
        </span>
        <span className="account-name">{user.username}</span>
        <ChevronDown aria-hidden="true" />
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuGroup>
          <DropdownMenuLabel className="account-label">
            <strong>{user.username}</strong>
            <small>{user.admin ? "Administrator" : "Uploader"}</small>
          </DropdownMenuLabel>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuLabel>Theme</DropdownMenuLabel>
          <DropdownMenuRadioGroup
            value={theme}
            onValueChange={(value) => onTheme(value as Theme)}
          >
            {themes.map(([value, label, Icon]) => (
              <DropdownMenuRadioItem key={value} value={value}>
                <Icon aria-hidden="true" />
                {label}
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem data-action="logout" onClick={onLogout}>
          <LogOut aria-hidden="true" />
          Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
