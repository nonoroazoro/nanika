export interface SearchObservation
{
    requestId: number;
    stage: "committed" | "failed" | "input" | "received" | "rejected";
    time: number;
}
