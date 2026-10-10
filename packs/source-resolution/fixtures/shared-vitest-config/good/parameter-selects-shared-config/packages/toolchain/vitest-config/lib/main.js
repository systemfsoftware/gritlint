export const sourceCondition = '@systemfsoftware/source'

export const sharedConfig = {
  ssr: { resolve: { conditions: [sourceCondition] } },
}
