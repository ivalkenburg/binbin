import { useId, type ComponentProps, type ReactNode } from "react";
import { Input } from "./ui/input";
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
