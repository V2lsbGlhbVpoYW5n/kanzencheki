export type ID = string;
export interface Person { id: ID; name: string }
export interface Cheki {
  id: ID;
  title: string;
  takenOn: string | null;
  personIds: ID[];
  notes: string;
  status: 'inbox' | 'organized';
  coverAssetId: ID | null;
}
export interface Asset {
  id: ID;
  kind: 'scanner' | 'phone' | 'scene';
  capturedAt: string | null;
}
export interface ChekiAsset { chekiId: ID; assetId: ID }
export interface Rendition {
  id: ID;
  assetId: ID;
  role: 'original' | 'display' | 'thumbnail';
  relativePath: string;
  originalFilename: string;
  mimeType: string;
  byteSize: number;
  width: number | null;
  height: number | null;
}
export interface Library {
  chekis: Cheki[];
  people: Person[];
  assets: Asset[];
  chekiAssets: ChekiAsset[];
  renditions: Rendition[];
}
