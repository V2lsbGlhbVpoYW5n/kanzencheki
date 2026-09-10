export interface Photo {
  id: string;
  title: string;
  date: string;
  tag: string;
  src: string;
  images?: { id: string; src: string; name: string }[];
  source?: string;
  favorite: boolean;
  inbox: boolean;
  crop?: { x: number; y: number; w: number; h: number };
  notes: string;
}
export const demoPhotos: Photo[] = [
  {
    id: "01",
    title: "粉色演出服",
    date: "2026-08-27",
    tag: "演出",
    src: "/demo/signed.jpg",
    source: "https://note.com/timelmitei/n/n1a86110c5b5a",
    favorite: true,
    inbox: false,
    notes: "",
  },
  {
    id: "02",
    title: "红色签名 · 01",
    date: "2026-08-27",
    tag: "演出",
    src: "/demo/pair.jpg",
    source: "https://note.com/otaku_znpn_ymmt/n/na131be42e611",
    favorite: false,
    inbox: false,
    crop: { x: 20, y: 24, w: 30, h: 47 },
    notes: "",
  },
  {
    id: "03",
    title: "红色签名 · 02",
    date: "2026-08-27",
    tag: "演出",
    src: "/demo/pair.jpg",
    source: "https://note.com/otaku_znpn_ymmt/n/na131be42e611",
    favorite: true,
    inbox: false,
    crop: { x: 54, y: 24, w: 31, h: 46 },
    notes: "",
  },
  {
    id: "04",
    title: "白色蕾丝",
    date: "2026-08-27",
    tag: "咖啡店",
    src: "/demo/collection.jpg",
    source: "https://booth.pm/ja/items/5802885",
    favorite: false,
    inbox: false,
    crop: { x: 15, y: 16, w: 34, h: 40 },
    notes: "",
  },
  {
    id: "05",
    title: "黑色蝴蝶结",
    date: "2026-08-16",
    tag: "咖啡店",
    src: "/demo/collection.jpg",
    source: "https://booth.pm/ja/items/5802885",
    favorite: true,
    inbox: false,
    crop: { x: 48, y: 9, w: 31, h: 44 },
    notes: "",
  },
  {
    id: "06",
    title: "生日纪念",
    date: "2026-08-16",
    tag: "纪念",
    src: "/demo/birthday.jpg",
    source: "https://asobisystem.shop/products/KLM-0016",
    favorite: false,
    inbox: false,
    crop: { x: 43, y: 32, w: 22, h: 30 },
    notes: "",
  },
  {
    id: "07",
    title: "白色贝雷帽",
    date: "2026-08-16",
    tag: "纪念",
    src: "/demo/birthday.jpg",
    source: "https://asobisystem.shop/products/KLM-0016",
    favorite: false,
    inbox: true,
    crop: { x: 29, y: 53, w: 20, h: 28 },
    notes: "",
  },
  {
    id: "08",
    title: "一起喝杯咖啡",
    date: "2026-08-16",
    tag: "返切",
    src: "/demo/cafe.png",
    source:
      "https://www.codigonuevo.com/yo/moda/el-amor-por-las-camaras-analogicas-es-real-asi-es-la-instax-mini-40-JX2000913",
    favorite: false,
    inbox: true,
    notes: "",
  },
];
