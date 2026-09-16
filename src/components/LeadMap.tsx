import { useEffect, useRef } from "react";
import L from "leaflet";
import "leaflet/dist/leaflet.css";
import type { Lead } from "../types";

type Props = {
  center: [number, number] | null;
  radiusKm: number;
  leads: Lead[];
};

export default function LeadMap({ center, radiusKm, leads }: Props) {
  const divRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<L.Map | null>(null);

  useEffect(() => {
    if (!divRef.current || mapRef.current) return;
    const map = L.map(divRef.current, { zoomControl: true }).setView(center ?? [-15.78, -47.92], center ? 12 : 4);
    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
      attribution: "© OpenStreetMap",
      maxZoom: 19,
    }).addTo(map);
    mapRef.current = map;
    return () => {
      map.remove();
      mapRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;
    if (center) {
      map.setView(center, radiusKm > 15 ? 10 : radiusKm > 7 ? 12 : 13);
    }
  }, [center, radiusKm]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;
    const layer = L.layerGroup().addTo(map);
    if (center) {
      L.circle(center, { radius: radiusKm * 1000, color: "#16a34a", weight: 1, fillOpacity: 0.05 }).addTo(layer);
      L.marker(center).addTo(layer).bindPopup("Centro da busca");
    }
    leads.slice(0, 200).forEach((l) => {
      if (l.latitude == null || l.longitude == null) return;
      const color = (l.score ?? 0) >= 60 ? "#16a34a" : (l.score ?? 0) >= 30 ? "#ca8a04" : "#525252";
      L.circleMarker([l.latitude, l.longitude], {
        radius: 6,
        color: "#fff",
        weight: 1.5,
        fillColor: color,
        fillOpacity: 0.9,
      })
        .addTo(layer)
        .bindPopup(`<b>${l.canonical_name}</b><br/>${l.address ?? ""}<br/>Score ${l.score ?? 0}`);
    });
    return () => {
      layer.remove();
    };
  }, [leads, center, radiusKm]);

  return <div ref={divRef} className="h-[320px] w-full overflow-hidden rounded-xl border border-neutral-200" />;
}
