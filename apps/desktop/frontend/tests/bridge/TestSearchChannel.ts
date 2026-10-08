import type { RootSearchSnapshot } from "../../src/generated/RootSearchSnapshot";

export class TestSearchChannel
{
    static readonly instances: TestSearchChannel[] = [];

    onmessage: (update: RootSearchSnapshot) => void;

    constructor(listener: (update: RootSearchSnapshot) => void)
    {
        this.onmessage = listener;
        TestSearchChannel.instances.push(this);
    }
}
