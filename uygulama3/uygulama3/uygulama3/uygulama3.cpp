#include <iostream>
#include <fstream>
#include <string>
#include <vector>
#include <algorithm>
#include <iomanip>
#include <map>
#include <windows.h>
#include <shlwapi.h>

#pragma comment(lib, "shlwapi.lib")

class FolderBatchSearcher {
private:
    std::string folderPath;
    std::string searchString;

    struct SearchResult {
        std::string fileName;
        std::string fullPath;
        bool found;
        size_t occurrences;
        std::vector<size_t> positions;
        size_t fileSize;
    };

    std::vector<SearchResult> results;

    // String'i k���k harfe �evir (case-insensitive arama i�in)
    std::string toLower(const std::string& str) {
        std::string result = str;
        std::transform(result.begin(), result.end(), result.begin(), ::tolower);
        return result;
    }

    // Dosya boyutunu al
    size_t getFileSize(const std::string& fileName) {
        WIN32_FIND_DATAA findData;
        HANDLE hFind = FindFirstFileA(fileName.c_str(), &findData);
        if (hFind == INVALID_HANDLE_VALUE) {
            return 0;
        }

        LARGE_INTEGER fileSize;
        fileSize.LowPart = findData.nFileSizeLow;
        fileSize.HighPart = findData.nFileSizeHigh;
        FindClose(hFind);

        return static_cast<size_t>(fileSize.QuadPart);
    }

    // Tek bir dosyada arama yap
    SearchResult searchInFile(const std::string& filePath, const std::string& fileName) {
        SearchResult result;
        result.fileName = fileName;
        result.fullPath = filePath;
        result.found = false;
        result.occurrences = 0;
        result.fileSize = getFileSize(filePath);

        std::ifstream file(filePath, std::ios::binary);
        if (!file.is_open()) {
            return result;
        }

        // Bo� dosya kontrol�
        if (result.fileSize == 0) {
            file.close();
            return result;
        }

        // Buffer boyutu
        const size_t BUFFER_SIZE = 4096;
        std::vector<char> buffer(BUFFER_SIZE + searchString.length() - 1);

        size_t totalBytesRead = 0;
        std::string searchLower = toLower(searchString);

        while (!file.eof()) {
            std::fill(buffer.begin(), buffer.end(), 0);

            file.read(buffer.data(), BUFFER_SIZE);
            size_t bytesRead = static_cast<size_t>(file.gcount());

            if (bytesRead == 0) break;

            std::string bufferStr(buffer.data(), bytesRead + searchString.length() - 1);
            std::string bufferLower = toLower(bufferStr);

            size_t pos = 0;
            while ((pos = bufferLower.find(searchLower, pos)) != std::string::npos) {
                size_t actualAddress = totalBytesRead + pos;
                result.positions.push_back(actualAddress);
                result.found = true;
                result.occurrences++;
                pos++;
            }

            totalBytesRead += bytesRead;

            // Overlap i�in geri git
            if (!file.eof() && bytesRead >= searchString.length()) {
                file.seekg(-(static_cast<std::streamoff>(searchString.length() - 1)), std::ios::cur);
                totalBytesRead -= (searchString.length() - 1);
            }
        }

        file.close();
        return result;
    }

    // Dosya tipini belirle
    std::string getFileType(const std::string& fileName) {
        size_t dotPos = fileName.find_last_of('.');
        if (dotPos == std::string::npos) {
            return "Bilinmeyen";
        }

        std::string ext = fileName.substr(dotPos + 1);
        std::transform(ext.begin(), ext.end(), ext.begin(), ::tolower);

        if (ext == "exe" || ext == "dll" || ext == "sys") return "Executable";
        if (ext == "txt" || ext == "log" || ext == "cfg") return "Text";
        if (ext == "doc" || ext == "docx" || ext == "pdf") return "Document";
        if (ext == "jpg" || ext == "png" || ext == "bmp") return "Image";
        if (ext == "mp3" || ext == "wav" || ext == "mp4") return "Media";

        return "Diger";
    }

    // Dosya boyutunu okunabilir formatta g�ster
    std::string formatFileSize(size_t bytes) {
        if (bytes < 1024) return std::to_string(bytes) + " B";
        if (bytes < 1024 * 1024) return std::to_string(bytes / 1024) + " KB";
        if (bytes < 1024 * 1024 * 1024) return std::to_string(bytes / (1024 * 1024)) + " MB";
        return std::to_string(bytes / (1024 * 1024 * 1024)) + " GB";
    }

public:
    FolderBatchSearcher(const std::string& folder, const std::string& search)
        : folderPath(folder), searchString(search) {
        // Klas�r yolunun sonuna \ ekle (yoksa)
        if (!folderPath.empty() && folderPath.back() != '\\') {
            folderPath += "\\";
        }
    }

    // Klas�rdeki dosyalar� listele ve ara
    bool searchInFolder() {
        std::string searchPattern = folderPath + "*";

        WIN32_FIND_DATAA findData;
        HANDLE hFind = FindFirstFileA(searchPattern.c_str(), &findData);

        if (hFind == INVALID_HANDLE_VALUE) {
            std::cerr << "Hata: Klasor '" << folderPath << "' acilamadi!" << std::endl;
            return false;
        }

        std::cout << "Klasor: " << folderPath << std::endl;
        std::cout << "Aranan: " << searchString << std::endl;
        std::cout << "========================================" << std::endl;

        int totalFiles = 0;
        int filesWithContent = 0;

        do {
            std::string fileName = findData.cFileName;

            // . ve .. klas�rlerini atla
            if (fileName == "." || fileName == "..") {
                continue;
            }

            // Sadece dosyalar� i�le (alt klas�rleri atla)
            if (!(findData.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY)) {
                std::string fullPath = folderPath + fileName;
                totalFiles++;

                std::cout << "\r[" << totalFiles << "] Taran�yor: " << fileName << std::flush;

                SearchResult result = searchInFile(fullPath, fileName);
                results.push_back(result);

                if (result.found) {
                    filesWithContent++;
                }
            }

        } while (FindNextFileA(hFind, &findData));

        FindClose(hFind);

        std::cout << "\r" << std::string(80, ' ') << "\r"; // Sat�r� temizle

        if (totalFiles == 0) {
            std::cout << "Klasorde dosya bulunamadi." << std::endl;
            return false;
        }

        // Sonu�lar� g�ster
        showResults();
        showSummary(totalFiles, filesWithContent);

        return filesWithContent > 0;
    }

    // Sonu�lar� detayl� g�ster
    void showResults() {
        std::cout << "\n=== ARAMA SONUCLARI ===" << std::endl;
        std::cout << std::left << std::setw(40) << "DOSYA ADI"
            << std::setw(10) << "DURUM"
            << std::setw(10) << "ADET"
            << std::setw(12) << "BOYUT"
            << std::setw(12) << "TIP" << std::endl;
        std::cout << std::string(84, '-') << std::endl;

        for (const auto& result : results) {
            std::string status = result.found ? "BULUNDU" : "YOK";
            std::string count = result.found ? std::to_string(result.occurrences) : "-";

            std::cout << std::left << std::setw(40) << result.fileName.substr(0, 39)
                << std::setw(10) << status
                << std::setw(10) << count
                << std::setw(12) << formatFileSize(result.fileSize)
                << std::setw(12) << getFileType(result.fileName) << std::endl;
        }
    }

    // Detayl� sonu�lar� g�ster (bulunan konumlar ile)
    void showDetailedResults() {
        std::cout << "\n=== DETAYLI SONUCLAR ===" << std::endl;

        for (const auto& result : results) {
            if (result.found) {
                std::cout << "\nDosya: " << result.fileName << std::endl;
                std::cout << "Yol: " << result.fullPath << std::endl;
                std::cout << "Bulunan konum sayisi: " << result.occurrences << std::endl;
                std::cout << "Konumlar: ";

                for (size_t i = 0; i < result.positions.size() && i < 10; ++i) {
                    std::cout << "0x" << std::hex << result.positions[i] << std::dec;
                    if (i < result.positions.size() - 1 && i < 9) {
                        std::cout << ", ";
                    }
                }

                if (result.positions.size() > 10) {
                    std::cout << " ... (+" << (result.positions.size() - 10) << " daha)";
                }
                std::cout << std::endl;
            }
        }
    }

    // �zet bilgileri g�ster
    void showSummary(int totalFiles, int filesWithContent) {
        std::cout << "\n=== OZET ===" << std::endl;
        std::cout << "Taranan klasor: " << folderPath << std::endl;
        std::cout << "Aranan string: " << searchString << std::endl;
        std::cout << "Toplam dosya sayisi: " << totalFiles << std::endl;
        std::cout << "'" << searchString << "' bulunan dosya sayisi: " << filesWithContent << std::endl;
        std::cout << "Bulunamayan dosya sayisi: " << (totalFiles - filesWithContent) << std::endl;

        // Dosya tipi istatistikleri
        std::map<std::string, int> typeStats;
        size_t totalSize = 0;

        for (const auto& result : results) {
            typeStats[getFileType(result.fileName)]++;
            totalSize += result.fileSize;
        }

        std::cout << "\nDosya tipi dagilimi:" << std::endl;
        for (const auto& stat : typeStats) {
            std::cout << "  " << stat.first << ": " << stat.second << " dosya" << std::endl;
        }

        std::cout << "Toplam tarama boyutu: " << formatFileSize(totalSize) << std::endl;

        double successRate = (totalFiles > 0) ? (double)filesWithContent / totalFiles * 100.0 : 0.0;
        std::cout << "Basari orani: %" << std::fixed << std::setprecision(1) << successRate << std::endl;
    }

    // Sonu�lar� dosyaya kaydet
    void saveResults(const std::string& outputFile) {
        std::ofstream out(outputFile);
        if (!out.is_open()) {
            std::cerr << "Hata: Cikti dosyasi olusturulamadi!" << std::endl;
            return;
        }

        out << "=== KLASOR BAZLI TOPLU ARAMA RAPORU ===" << std::endl;
        out << "Tarih: " << __DATE__ << " " << __TIME__ << std::endl;
        out << "Klasor: " << folderPath << std::endl;
        out << "Aranan: " << searchString << std::endl;
        out << std::endl;

        for (const auto& result : results) {
            out << "Dosya: " << result.fileName << std::endl;
            out << "Durum: " << (result.found ? "BULUNDU" : "BULUNAMADI") << std::endl;
            if (result.found) {
                out << "Adet: " << result.occurrences << std::endl;
            }
            out << "Boyut: " << formatFileSize(result.fileSize) << std::endl;
            out << "---" << std::endl;
        }

        out.close();
        std::cout << "\nRapor '" << outputFile << "' dosyasina kaydedildi." << std::endl;
    }
};

void showUsage(const char* programName) {
    std::cout << "=== KLASOR BAZLI TOPLU ARAMA ===" << std::endl;
    std::cout << "Windows 10/11 Uyumlu - Dis Kutuphanesi Gerektirmez" << std::endl;
    std::cout << std::endl;
    std::cout << "Kullanim:" << std::endl;
    std::cout << "  " << programName << " <klasor_yolu> [aranan_string]" << std::endl;
    std::cout << std::endl;
    std::cout << "Parametreler:" << std::endl;
    std::cout << "  klasor_yolu    : Aranacak klasorun tam yolu" << std::endl;
    std::cout << "  aranan_string  : Aranacak metin (varsayilan: MALWARE)" << std::endl;
    std::cout << std::endl;
    std::cout << "Ornekler:" << std::endl;
    std::cout << "  " << programName << " C:\\test\\" << std::endl;
    std::cout << "  " << programName << " C:\\test\\ MALWARE" << std::endl;
    std::cout << "  " << programName << " \"C:\\Program Files\\MyApp\\\" virus" << std::endl;
    std::cout << "  " << programName << " .\\test_folder suspicious" << std::endl;
    std::cout << std::endl;
    std::cout << "Not:" << std::endl;
    std::cout << "- Sadece belirtilen klasordeki dosyalar taranir (alt klasorler dahil edilmez)" << std::endl;
    std::cout << "- Arama case-insensitive (buyuk/kucuk harf duyarsiz) yapilir" << std::endl;
    std::cout << "- Her dosya icin bulundu/bulunamadi durumu raporlanir" << std::endl;
}

int main(int argc, char* argv[]) {
    // Windows konsol ayarlar�
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);

    std::cout << "=== KLASOR BAZLI TOPLU ARAMA v1.0 ===" << std::endl;
    std::cout << "Windows Uyumlu Klasor Tarama Araci" << std::endl;
    std::cout << "=====================================" << std::endl << std::endl;

    // Parametre kontrol�
    if (argc < 2) {
        showUsage(argv[0]);
        return 1;
    }

    std::string folderPath = argv[1];
    std::string searchString = (argc >= 3) ? argv[2] : "MALWARE";

    // Klas�r varl���n� kontrol et
    DWORD dwAttrib = GetFileAttributesA(folderPath.c_str());

    if (dwAttrib == INVALID_FILE_ATTRIBUTES) {
        std::cerr << "Hata: '" << folderPath << "' klasoru bulunamadi!" << std::endl;
        return 2;
    }

    if (!(dwAttrib & FILE_ATTRIBUTE_DIRECTORY)) {
        std::cerr << "Hata: '" << folderPath << "' bir klasor degil!" << std::endl;
        return 3;
    }

    // Arama stringinin ge�erlili�ini kontrol et
    if (searchString.empty()) {
        std::cerr << "Hata: Aranan string bos olamaz!" << std::endl;
        return 4;
    }

    // FolderBatchSearcher nesnesini olu�tur ve aramay� ba�lat
    FolderBatchSearcher searcher(folderPath, searchString);

    std::cout << "Klasor bazli toplu arama baslatiiliyor..." << std::endl;
    std::cout << "Not: Alt klasorler taranmayacak (recursive degil)" << std::endl << std::endl;

    // Arama i�lemini ger�ekle�tir
    bool result = searcher.searchInFolder();

    // Kullan�c�ya detayl� sonu� isteyip istemedi�ini sor
    std::cout << "\nDetayli sonuclari gormek istiyor musunuz? (y/n): ";
    char choice;
    std::cin >> choice;

    if (choice == 'y' || choice == 'Y') {
        searcher.showDetailedResults();
    }

    // Rapor kaydetme se�ene�i
    std::cout << "\nSonuclari dosyaya kaydetmek istiyor musunuz? (y/n): ";
    std::cin >> choice;

    if (choice == 'y' || choice == 'Y') {
        std::string outputFile = "arama_raporu.txt";
        searcher.saveResults(outputFile);
    }

    std::cout << std::endl << "=====================================" << std::endl;
    std::cout << "Klasor tarama islemi tamamlandi." << std::endl;

    return 0;
}

// Derleme talimatlar�:
// g++ -std=c++11 -O2 -o folder_batch_searcher.exe folder_batch_searcher.cpp -lshlwapi
// 
// Veya Microsoft Visual Studio ile:
// cl /EHsc /O2 folder_batch_searcher.cpp shlwapi.lib
//
// Kullanim ornekleri:
// folder_batch_searcher.exe C:\test\
// folder_batch_searcher.exe C:\test\ MALWARE
// folder_batch_searcher.exe "C:\Program Files\MyApp\" virus