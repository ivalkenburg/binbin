import { useId, type ComponentProps, type ReactNode } from "react";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import { Field as FieldRoot, FieldLabel } from "./ui/field";

export function Field({
  label,
  id,
  ...props
}: ComponentProps<typeof Input> & { label: ReactNode }) {
  const generated = useId();
  return (
    <FieldRoot data-disabled={props.disabled || undefined}>
      <FieldLabel htmlFor={id || generated}>{label}</FieldLabel>
      <Input id={id || generated} {...props} />
    </FieldRoot>
  );
}
export function CheckField({
  label,
  ...props
}: ComponentProps<typeof Checkbox> & { label: string }) {
  const id = useId();
  return (
    <FieldRoot
      orientation="horizontal"
      data-disabled={props.disabled || undefined}
    >
      <Checkbox id={id} {...props} />
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
    </FieldRoot>
  );
}
export interface Preset {
  value: string;
  label: string;
}
export function PresetField({
  label,
  presets,
  value,
  onValueChange,
  ...props
}: Omit<ComponentProps<typeof Input>, "value" | "onChange"> & {
  label: ReactNode;
  presets: Preset[];
  value: string;
  onValueChange: (value: string) => void;
}) {
  const id = useId();
  return (
    <FieldRoot data-disabled={props.disabled || undefined}>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Input
        id={id}
        value={value}
        onChange={(event) => onValueChange(event.target.value)}
        {...props}
      />
      <div
        className="preset-chips"
        role="group"
        aria-label={`${props.name} presets`}
      >
        {presets.map((preset) => (
          <Button
            key={preset.label}
            type="button"
            size="xs"
            variant={value === preset.value ? "secondary" : "outline"}
            aria-pressed={value === preset.value}
            disabled={props.disabled}
            onClick={() => onValueChange(preset.value)}
          >
            {preset.label}
          </Button>
        ))}
      </div>
    </FieldRoot>
  );
}
