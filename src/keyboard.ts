export interface EscapeContext { defaultPrevented:boolean; composing:boolean; terminal:boolean; editing:boolean; dialog:boolean }

export function shouldCloseOnEscape(key:string, context:EscapeContext):boolean {
  return key==='Escape' && !context.defaultPrevented && !context.composing
    && !context.terminal && !context.editing && !context.dialog;
}
