export interface V<out T extends object, in U, in out W> { readonly v: T; f: (u: U) => W }
