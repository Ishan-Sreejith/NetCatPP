import SwiftUI

struct StyledTextField: View {
    @Binding var text: String
    let placeholder: String
    var width: CGFloat?
    var height: CGFloat?
    
    init(text: Binding<String>, placeholder: String, width: CGFloat? = nil, height: CGFloat? = nil) {
        self._text = text
        self.placeholder = placeholder
        self.width = width
        self.height = height
    }
    
    var body: some View {
        TextField(placeholder, text: $text)
            .textFieldStyle(.plain)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(AppTheme.tertiaryBackground)
            .cornerRadius(8)
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(AppTheme.border, lineWidth: 1)
            )
            .frame(width: width, height: height)
    }
}