export interface ViewAction
{
    id: string;
    title: string;
    confirmation_title?: string;
    style: "destructive" | "primary" | "secondary";
}
