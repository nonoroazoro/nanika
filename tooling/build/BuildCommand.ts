export type BuildCommand = (
    command: string[],
    cwd?: string,
    environment?: Record<string, string | undefined>
) => Promise<void>;
