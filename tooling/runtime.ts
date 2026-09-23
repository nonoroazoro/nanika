const manifest = await Bun.file(new URL("../package.json", import.meta.url)).json() as {
    packageManager: string;
};
if (`bun@${Bun.version}` !== manifest.packageManager)
{
    throw new Error(`Expected ${manifest.packageManager}; found bun@${Bun.version}.`);
}
