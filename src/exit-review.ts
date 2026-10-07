export interface ExitResource {
  kind:string;id:string;label:string;provider:string;account:string;billing:string;model:string;
  cwd:string|null;revision:number|null;sendNonce:string|null;
}
export interface ExitReview {nonce:string;expiresAt:number;resources:ExitResource[]}
