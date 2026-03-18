import re
from dataclasses import dataclass
from typing import List, Optional


@dataclass
class ChunkInfo:
    content: str
    char_start: int
    char_end: int
    index: int


class ChineseChunker:
    def __init__(
        self,
        max_chunk_size: int = 400,
        min_chunk_size: int = 50,
        overlap: int = 0,
    ):
        self.max_chunk_size = max_chunk_size
        self.min_chunk_size = min_chunk_size
        self.overlap = overlap

    def chunk_text(self, text: str) -> List[ChunkInfo]:
        paragraphs = self._split_paragraphs(text)

        chunks: List[ChunkInfo] = []
        current_pos = 0
        chunk_index = 0

        for para in paragraphs:
            para_start = text.find(para, current_pos)
            if para_start == -1:
                continue

            para_end = para_start + len(para)
            current_pos = para_end

            if len(para.strip()) < self.min_chunk_size:
                if chunks and len(chunks[-1].content) + len(para) < self.max_chunk_size:
                    last_chunk = chunks[-1]
                    chunks[-1] = ChunkInfo(
                        content=last_chunk.content + "\n" + para,
                        char_start=last_chunk.char_start,
                        char_end=para_end,
                        index=last_chunk.index,
                    )
                    continue

            if len(para) > self.max_chunk_size:
                sub_chunks = self._split_long_paragraph(para, para_start)
                for sub in sub_chunks:
                    sub.index = chunk_index
                    chunks.append(sub)
                    chunk_index += 1
            else:
                chunks.append(
                    ChunkInfo(
                        content=para.strip(),
                        char_start=para_start,
                        char_end=para_end,
                        index=chunk_index,
                    )
                )
                chunk_index += 1

        for i, chunk in enumerate(chunks):
            chunk.index = i

        return chunks

    def _split_paragraphs(self, text: str) -> List[str]:
        paragraphs = re.split(r"\n\s*\n|\n", text)
        return [p for p in paragraphs if p.strip()]

    def _split_long_paragraph(self, para: str, para_start: int) -> List[ChunkInfo]:
        sentences = self._split_sentences(para)
        chunks: List[ChunkInfo] = []

        current_chunk = ""
        current_start = para_start

        for sentence in sentences:
            if len(current_chunk) + len(sentence) > self.max_chunk_size:
                if current_chunk:
                    chunks.append(
                        ChunkInfo(
                            content=current_chunk.strip(),
                            char_start=current_start,
                            char_end=current_start + len(current_chunk),
                            index=0,
                        )
                    )
                    current_start = current_start + len(current_chunk)
                    current_chunk = ""

            current_chunk += sentence

        if current_chunk.strip():
            chunks.append(
                ChunkInfo(
                    content=current_chunk.strip(),
                    char_start=current_start,
                    char_end=current_start + len(current_chunk),
                    index=0,
                )
            )

        return chunks

    def _split_sentences(self, text: str) -> List[str]:
        pattern = r"([^。！？.!?\n]+[。！？.!?\n]?)"
        sentences = re.findall(pattern, text)
        return sentences
