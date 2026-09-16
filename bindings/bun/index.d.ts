export type InputFormat = "d2" | "mermaid";
export type OutputFormat = "d2" | "mermaid";

export class Document {
  constructor(source: string, inputFormat: InputFormat);
  transform(query: string, outputFormat: OutputFormat): string;
}

export function transform(
  source: string,
  query: string,
  inputFormat: InputFormat,
  outputFormat: OutputFormat,
): string;
